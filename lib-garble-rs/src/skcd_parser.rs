use crate::circuit::{
    DisplayConfig, EvaluatorInputs, EvaluatorInputsType, GarblerInputs, GarblerInputsType,
    InterstellarCircuit, SkcdConfig,
};
use fancy_garbling::circuit::CircuitBuilder;
use fancy_garbling::circuit::CircuitRef;
use fancy_garbling::Fancy;
use num_enum::TryFromPrimitive;
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
mod interstellarpbskcd {
    // TODO(interstellar) can we use prost-build(and prost-derive) in SGX env?
    // include!(concat!(env!("OUT_DIR"), "/interstellarpbskcd.rs"));
    include!("../deps/protos/generated/rust/interstellarpbskcd.rs");
}

/// All the Gates type possible in SKCD file format
///
/// SHOULD match
/// - "enum SkcdGateType" from skcd.proto
/// - lib_circuits/src/blif/gate_types.h
/// - lib_garble/src/justgarble/gate_types.h
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, TryFromPrimitive)]
#[repr(i32)]
enum SkcdGateType {
    ZERO = 0,
    NOR = 1,
    /// A-and-not-B
    AANB = 2,
    /// NOT B
    INVB = 3,
    /// not-A-and-B?
    NAAB = 4,
    /// NOT A
    INV = 5,
    XOR = 6,
    NAND = 7,
    AND = 8,
    XNOR = 9,
    BUF = 10,
    /// A-or-NOT-B?
    AONB = 11,
    BUFB = 12,
    /// NOT-A-or-B?
    NAOB = 13,
    OR = 14,
    ONE = 15,
}

/// Errors emitted by the circuit parser.
#[derive(Debug)]
pub enum CircuitParserError {
    /// InvalidGateIdError: the given GateID from the .skcd DOES NOT match CircuitBuilder's
    InvalidGateId(String),
    /// Can not convert GarblerInputsType(i32) to GarblerInputsType
    GarblerInputsType,
    /// Can not convert EvaluatorInputsType(i32) to EvaluatorInputsType
    EvaluatorInputsType,
}

impl InterstellarCircuit {
    /// Parse a Protobuf-serialized .skcd file
    /// It is doing what fancy-garbling/src/parser.rs is doing for a "Blif Fashion" txt file,
    /// but for a .skcd instead.
    /// SKCD is essentially the same format, but with the gates written in a different order:
    /// - in "Bilf Fashion": gates are written "gate0_input0 gate0_input1 gate0_output gate0_type" etc
    /// - in SKCD: "gate0_input0 gate1_input0 gate2_input0" etc
    ///
    /// return:
    /// - the graph corresponding to the .skcd(as-is; gates NOT transformed/optimized/etc)
    /// - the list of inputs (gate ids)
    /// - the list of ouputs (gate ids)
    /// [inputs/outputs are needed to walk the graph, and optimize/rewrite if desired]
    pub(crate) fn parse_skcd(buf: &[u8]) -> Result<InterstellarCircuit, CircuitParserError> {
        // TODO(interstellar) decode_length_delimited ?
        let skcd: interstellarpbskcd::Skcd = prost::Message::decode(buf).unwrap();

        let mut circ_builder = CircuitBuilder::new();

        // TODO(interstellar) modulus: what should we use??
        let q = 2;

        let mut skcd_gate_converter = SkcdGateConverter::new();

        let skcd_config = skcd.config.unwrap();
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

        for (idx, skcd_input) in skcd.inputs.iter().enumerate() {
            let new_gate = if skcd_inputs_is_garbled[idx] {
                circ_builder.garbler_input(q)
            } else {
                circ_builder.evaluator_input(q)
            };
            skcd_gate_converter.insert(skcd_input, new_gate);
        }

        // TODO(interstellar) how should we use skcd's a/b/go?
        for skcd_gate in skcd.gates {
            let xref = skcd_gate_converter.get(&skcd_gate.a);
            let yref = skcd_gate_converter.get(&skcd_gate.b);

            let new_gate = match skcd_gate.r#type.try_into() {
                Ok(SkcdGateType::ZERO) => circ_builder.constant(0, q).unwrap(),
                Ok(SkcdGateType::ONE) => circ_builder.constant(1, q).unwrap(),
                Ok(SkcdGateType::OR) => {
                    // TODO can we use fn proj(&mut self, A: &Wire, q_out: u16, tt: Option<Vec<u16>>)?
                    // let z = circ_builder.proj(&xref, q, Some(vec![0; 4]));
                    circ_builder.or(xref?, yref?).unwrap()
                }
                Ok(SkcdGateType::XOR) => {
                    // TODO can we use fn proj(&mut self, A: &Wire, q_out: u16, tt: Option<Vec<u16>>)?
                    // let z = circ_builder.proj(&xref, q, Some(vec![0; 4]));
                    circ_builder.xor(xref?, yref?).unwrap()
                }
                Ok(SkcdGateType::NAND) => {
                    // TODO can we use fn proj(&mut self, A: &Wire, q_out: u16, tt: Option<Vec<u16>>)?
                    // let z = circ_builder.proj(&xref, q, Some(vec![0; 4]));
                    let z = circ_builder.and(xref?, yref?).unwrap();
                    circ_builder.negate(&z).unwrap()
                }
                Ok(SkcdGateType::INV) => circ_builder.negate(xref?).unwrap(),
                Ok(SkcdGateType::BUF) => circ_builder.cmul(xref?, 1).unwrap(),
                _ => todo!(),
            };

            skcd_gate_converter.insert(&skcd_gate.o, new_gate);
        }

        // IMPORTANT: we MUST use skcd.o to set the CORRECT outputs
        // eg for the 2 bits adder.skcd:
        // - skcd.m = 1
        // - skcd.o = [8,11]
        // -> the 2 CORRECT outputs to be set are: [8,11]
        // If we set the bad ones, we get "FancyError::UninitializedValue" in fancy-garbling/src/circuit.rs at "fn eval"
        // eg L161 etc b/c the cache is not properly set
        for o in skcd.outputs {
            let z = skcd_gate_converter.get(&o).unwrap();
            circ_builder.output(z).unwrap();
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
            })
        }

        Ok(InterstellarCircuit {
            circuit: circ_builder.finish(),
            config,
        })
    }
}

/// We need to convert something like
/// ".gate XOR  a=rnd[2] b=rnd[0] O=n7016" in the .skcd(which is basically a .blif)
/// into something that CircuitBuilder can accept.
/// Essentially we need to convert a String ID -> CircuitRef(= a usize)
///
/// IMPORTANT
/// For this to work, the INPUTS MUST also go through the same conversion, else
/// when using CircuitBuilder.or/and/etc the CircuitRef WOULD NOT match anything.
/// NOTE that in this case the Circuit still would build fine, but it would fail
/// when eval/garbling.
struct SkcdGateConverter {
    map_skcd_gate_id_to_circuit_ref: HashMap<String, CircuitRef>,
}

impl SkcdGateConverter {
    pub fn new() -> Self {
        Self {
            map_skcd_gate_id_to_circuit_ref: HashMap::new(),
        }
    }

    pub fn get(&self, skcd_gate_id: &str) -> Result<&CircuitRef, CircuitParserError> {
        self.map_skcd_gate_id_to_circuit_ref
            .get(skcd_gate_id)
            .ok_or_else(|| CircuitParserError::InvalidGateId(skcd_gate_id.to_string()))
    }

    pub fn insert(&mut self, skcd_gate_id: &str, circuit_ref: CircuitRef) {
        self.map_skcd_gate_id_to_circuit_ref
            .insert(skcd_gate_id.to_string(), circuit_ref);
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

        assert!(circ.circuit.num_evaluator_inputs() == 3);
        for (i, inputs) in FULL_ADDER_2BITS_ALL_INPUTS.iter().enumerate() {
            let outputs = circ.circuit.eval_plain(&[], inputs).unwrap();
            assert_eq!(outputs, FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS[i]);
        }
    }
}
