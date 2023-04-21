use crate::circuit::{
    Circuit, DisplayConfig, EvaluatorInputs, EvaluatorInputsType, GarblerInputs, GarblerInputsType,
    GateType, InterstellarCircuit, SkcdConfig,
};
use crate::circuit::{Gate, GateInternal, GateRef};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::convert::TryInto;

#[cfg(all(not(feature = "std"), feature = "sgx"))]
extern crate sgx_tstd as std;

#[cfg(all(not(feature = "std"), feature = "sgx"))]
use sgx_tstd::string::String;

#[cfg(all(not(feature = "std"), feature = "sgx"))]
use sgx_tstd::vec::Vec;

#[cfg(all(not(feature = "std"), feature = "sgx"))]
use sgx_tstd::string::ToString;

// derive_partial_eq_without_eq: https://github.com/neoeinstein/protoc-gen-prost/issues/26
#[allow(clippy::derive_partial_eq_without_eq)]
#[allow(clippy::perf)]
#[allow(clippy::pedantic)]
mod interstellarpbskcd {
    // TODO(interstellar) can we use prost-build(and prost-derive) in SGX env?
    // include!(concat!(env!("OUT_DIR"), "/interstellarpbskcd.rs"));
    include!("../deps/protos/generated/rust/interstellarpbskcd.rs");
}

/// Errors emitted by the circuit parser.
#[derive(Debug)]
pub enum CircuitParserError {
    /// InvalidGateIdError: the given GateID from the .skcd DOES NOT match CircuitBuilder's
    InvalidGateId {
        gate_id: String,
    },
    /// Can not convert GarblerInputsType(i32) to GarblerInputsType
    GarblerInputsType,
    /// Can not convert EvaluatorInputsType(i32) to EvaluatorInputsType
    EvaluatorInputsType,
    DeserializerInternalError {
        err: prost::DecodeError,
    },
    /// Protobuf(proto3) field are NOT optional but on prost side they are Option<>
    /// so we need to add unwrap
    Proto3ProstOptionPlaceholder,
    /// when building the `outputs`, the GateID MUST have be created earlier
    /// This is a "sub-enum" of InvalidGateId
    OutputInvalidGateId {
        gate_id: String,
    },
}

impl InterstellarCircuit {
    /// Parse a Protobuf-serialized .skcd file
    /// It is doing what fancy-garbling/src/parser.rs is doing for a "Blif Fashion" txt file,
    /// but for a .skcd instead.
    /// SKCD is essentially the same format, but with the gates written in a different order:
    /// - in "Blif Fashion": gates are written `gate0_input0 gate0_input1 gate0_output gate0_type` etc
    /// - in SKCD: `gate0_input0 gate1_input0 gate2_input0` etc
    ///
    /// return:
    /// - the graph corresponding to the .skcd(as-is; gates NOT transformed/optimized/etc)
    /// - the list of inputs (gate ids)
    /// - the list of ouputs (gate ids)
    /// [inputs/outputs are needed to walk the graph, and optimize/rewrite if desired]
    #[allow(clippy::too_many_lines)]
    pub(crate) fn parse_skcd(buf: &[u8]) -> Result<InterstellarCircuit, CircuitParserError> {
        // TODO(interstellar) decode_length_delimited ?
        let skcd: interstellarpbskcd::Skcd = prost::Message::decode(buf)
            .map_err(|err| CircuitParserError::DeserializerInternalError { err })?;

        let mut skcd_gate_converter = SkcdGateConverter::new();

        let skcd_config = skcd
            .config
            .ok_or(CircuitParserError::Proto3ProstOptionPlaceholder)?;
        let mut input_idx = 0;

        // garbler inputs; same as "evaluator_inputs" above but a bit more complicated b/c how we are about to use
        // them depend on the type (eg 7 segments, i-ching, basic gate inputs for adder, etc)
        let mut skcd_inputs_is_garbled = Vec::<bool>::new();
        let mut garbler_inputs = Vec::with_capacity(skcd_config.garbler_inputs.len());
        for skcd_garbler_input in skcd_config.garbler_inputs {
            for _i in 0..skcd_garbler_input.length {
                skcd_inputs_is_garbled.push(true);
                input_idx += 1;
            }

            garbler_inputs.push(GarblerInputs {
                r#type: GarblerInputsType::try_from(skcd_garbler_input.r#type)
                    .map_err(|_e| CircuitParserError::GarblerInputsType)?,
                length: skcd_garbler_input.length,
            });
        }

        let mut evaluator_inputs = Vec::with_capacity(skcd_config.evaluator_inputs.len());
        for skcd_evaluator_input in skcd_config.evaluator_inputs {
            for _i in 0..skcd_evaluator_input.length {
                skcd_inputs_is_garbled.push(false);
                input_idx += 1;
            }

            evaluator_inputs.push(EvaluatorInputs {
                r#type: EvaluatorInputsType::try_from(skcd_evaluator_input.r#type)
                    .map_err(|_e| CircuitParserError::EvaluatorInputsType)?,
                length: skcd_evaluator_input.length,
            });
        }

        assert_eq!(
            input_idx,
            skcd.inputs.len(),
            "inputs and SkcdConfig fields DO NOT match!"
        );

        for skcd_input in skcd.inputs.iter() {
            skcd_gate_converter.insert(skcd_input);
        }

        // IMPORTANT: we MUST use skcd.o to set the CORRECT outputs
        // eg for the 2 bits adder.skcd:
        // - skcd.m = 1
        // - skcd.o = [8,11]
        // -> the 2 CORRECT outputs to be set are: [8,11]
        // If we set the bad ones, we get "FancyError::UninitializedValue" in fancy-garbling/src/circuit.rs at "fn eval"
        // eg L161 etc b/c the cache is not properly set
        for skcd_output in skcd.outputs.iter() {
            skcd_gate_converter.insert(skcd_output);
        }

        // TODO(interstellar) how should we use skcd's a/b/go?
        let mut gates = Vec::<Gate>::with_capacity(skcd.gates.len());
        for skcd_gate in skcd.gates {
            // **IMPORTANT** NOT ALL Gate to be built require x_ref and y_ref
            // so DO NOT unwrap here!
            // That would break the circuit building process!
            let x_ref =
                skcd_gate_converter
                    .get(&skcd_gate.a)
                    .ok_or(CircuitParserError::InvalidGateId {
                        gate_id: skcd_gate.a,
                    });
            let y_ref =
                skcd_gate_converter
                    .get(&skcd_gate.b)
                    .ok_or(CircuitParserError::InvalidGateId {
                        gate_id: skcd_gate.b,
                    });

            let new_gate_internal = match skcd_gate.r#type.try_into() {
                Ok(GateType::ZERO) => GateInternal::Constant { value: false },
                Ok(GateType::ONE) => GateInternal::Constant { value: true },
                Ok(GateType::INV) => GateInternal::Standard {
                    r#type: GateType::INV,
                    input_a: Some(x_ref?.clone()),
                    input_b: None,
                },
                Ok(GateType::BUF) => GateInternal::Standard {
                    r#type: GateType::BUF,
                    input_a: Some(x_ref?.clone()),
                    input_b: None,
                },
                Ok(skcd_gate_type) => GateInternal::Standard {
                    r#type: skcd_gate_type,
                    input_a: Some(x_ref?.clone()),
                    input_b: Some(y_ref?.clone()),
                },
                _ => todo!(),
            };

            skcd_gate_converter.insert(&skcd_gate.o);
            gates.push(Gate {
                internal: new_gate_internal,
                output: skcd_gate_converter
                    .get(&skcd_gate.o)
                    .ok_or(CircuitParserError::InvalidGateId {
                        gate_id: skcd_gate.o,
                    })?
                    .clone(),
            })
        }

        // config
        let mut config = SkcdConfig {
            display_config: None,
            garbler_inputs,
            evaluator_inputs,
        };
        // NOTE: "display_config" is OPTIONAL
        if let Some(skcd_display_config) = skcd_config.display_config {
            config.display_config = Some(DisplayConfig {
                width: skcd_display_config.width,
                height: skcd_display_config.height,
            });
        }

        // TODO
        // assert!(skcd.gates.len() == gates.len(), "invalid gates.len()!");

        Ok(InterstellarCircuit {
            circuit: Circuit {
                n: skcd.inputs.len(),
                m: skcd.outputs.len(),
                gates,
            },
            config,
        })
    }
}

/// We need to convert something like
/// ".gate XOR  a=rnd[2] b=rnd[0] O=n7016" in the .skcd(which is basically a .blif)
/// into something that `CircuitBuilder` can accept.
/// Essentially we need to convert a String ID -> `CircuitRef`(= a usize)
///
/// IMPORTANT
/// For this to work, the INPUTS MUST also go through the same conversion, else
/// when using CircuitBuilder.or/and/etc the `CircuitRef` WOULD NOT match anything.
/// NOTE that in this case the Circuit still would build fine, but it would fail
/// when eval/garbling.
struct SkcdGateConverter {
    map_skcd_gate_id_to_circuit_ref: HashMap<String, GateRef>,
    cur_len: usize,
}

impl SkcdGateConverter {
    pub fn new() -> Self {
        Self {
            map_skcd_gate_id_to_circuit_ref: HashMap::new(),
            cur_len: 0,
        }
    }

    pub fn get(&self, skcd_gate_id: &str) -> Option<&GateRef> {
        self.map_skcd_gate_id_to_circuit_ref.get(skcd_gate_id)
    }

    /// insert
    /// NOOP if already in the map
    pub fn insert(&mut self, skcd_gate_id: &str) {
        match self.get(skcd_gate_id) {
            Some(_) => {}
            None => {
                self.map_skcd_gate_id_to_circuit_ref
                    .insert(skcd_gate_id.to_string(), GateRef { id: self.cur_len });
                self.cur_len += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::circuit::InterstellarCircuit;
    use crate::tests::{FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS, FULL_ADDER_2BITS_ALL_INPUTS};

    #[test]
    fn test_eval_plain_full_adder_2bits() {
        let circ =
            InterstellarCircuit::parse_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin"))
                .unwrap();

        assert!(circ.num_evaluator_inputs() == 3);
        for (i, inputs) in FULL_ADDER_2BITS_ALL_INPUTS.iter().enumerate() {
            let outputs = circ.eval_plain(&[], inputs);
            assert_eq!(outputs, FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS[i]);
        }
    }
}
