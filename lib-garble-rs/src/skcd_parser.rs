use crate::circuit::WireRef;
use crate::circuit::{
    Circuit, CircuitInternal, CircuitMetadata, DisplayConfig, EvaluatorInputs, EvaluatorInputsType,
    GarblerInputs, GarblerInputsType, Gate, SkcdConfig, SkcdToWireRefConverter,
};
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::convert::TryFrom;
use hashbrown::HashSet;

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
    UnknownGateType {
        gate_type: i32,
    },
    /// `gate_type: Option<GateTypeBinary>` but it CAN(and WILL) be None only after serialization/deserialization
    InvalidStateGateTypeNotSet,
    /// For the "[constant gate special case]" we use the first input as a special "wire ID"
    ConstantSpecialCaseMissingInput,
    UnaryGateMissingInput,
    BinaryGateMissingInputA,
    BinaryGateMissingInputB,
}

impl Circuit {
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
    ///
    /// NOTE: due to the way the parsing is done(ie "inputs" first, then iterating on the "gates"
    /// WITH `InvalidGateId` if not yet present), the resulting Gates SHOULD
    /// be in topological(-ish) order.
    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_skcd(buf: &[u8]) -> Result<Circuit, CircuitParserError> {
        // TODO(interstellar) decode_length_delimited ?
        let skcd: interstellarpbskcd::Skcd = prost::Message::decode(buf)
            .map_err(|err| CircuitParserError::DeserializerInternalError { err })?;

        let mut skcd_to_wire_ref_converter = SkcdToWireRefConverter::new();

        let skcd_config = skcd
            .config
            .ok_or(CircuitParserError::Proto3ProstOptionPlaceholder)?;
        let mut input_idx = 0;

        // garbler inputs; same as "evaluator_inputs" above but a bit more complicated b/c how we are about to use
        // them depend on the type (eg 7 segments, i-ching, basic gate inputs for adder, etc)
        let mut skcd_inputs_is_garbled = Vec::<bool>::new();
        let mut garbler_inputs = Vec::with_capacity(skcd_config.garbler_inputs.len());
        let mut num_garbler_inputs = 0;
        for skcd_garbler_input in skcd_config.garbler_inputs {
            num_garbler_inputs += skcd_garbler_input.length;

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
        let mut num_evaluator_inputs = 0;
        for skcd_evaluator_input in skcd_config.evaluator_inputs {
            num_evaluator_inputs += skcd_evaluator_input.length;

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
            "inputs and SkcdConfig fields DO NOT match[1]!"
        );
        assert_eq!(
            num_garbler_inputs as usize + num_evaluator_inputs as usize,
            skcd.inputs.len(),
            "inputs and SkcdConfig fields DO NOT match[2]!"
        );

        let mut inputs = Vec::with_capacity(skcd.inputs.len());
        for skcd_input in &skcd.inputs {
            skcd_to_wire_ref_converter.insert(skcd_input);
            inputs.push(
                skcd_to_wire_ref_converter
                    .get(skcd_input)
                    .ok_or_else(|| CircuitParserError::InvalidGateId {
                        gate_id: skcd_input.to_string(),
                    })?
                    .clone(),
            );
        }

        // IMPORTANT: we MUST use skcd.o to set the CORRECT outputs
        // eg for the 2 bits adder.skcd:
        // - skcd.m = 1
        // - skcd.o = [8,11]
        // -> the 2 CORRECT outputs to be set are: [8,11]
        // If we set the bad ones, we get "FancyError::UninitializedValue" in fancy-garbling/src/circuit.rs at "fn eval"
        // eg L161 etc b/c the cache is not properly set
        let mut outputs = Vec::with_capacity(skcd.outputs.len());
        for skcd_output in &skcd.outputs {
            skcd_to_wire_ref_converter.insert(skcd_output);
            let output_wire_ref = skcd_to_wire_ref_converter
                .get(skcd_output)
                .ok_or_else(|| CircuitParserError::OutputInvalidGateId {
                    gate_id: skcd_output.to_string(),
                })?
                .clone();
            outputs.push(output_wire_ref);
        }
        // `outputs_start_end_indexes` after only works if `outputs` are consecutive; so CHECK it!
        // https://stackoverflow.com/questions/59028400/comparing-every-element-in-a-vector-with-the-next-one
        assert!(
            outputs.windows(2).all(|w| w[1].id == w[0].id + 1),
            "non consecutive elements in `outputs`!"
        );

        let outputs_clone = outputs.clone();
        let outputs_set: HashSet<&WireRef> = outputs_clone.iter().collect();

        // TODO? [constant gate special case]
        // we add two wires to represent constant 0 and 1
        // we loop just in case the wire ID would already be present in the "map"
        // let mut rng = ChaChaRng::from_entropy();
        // let wire_constant = generate_wire_with_fixed_id_and_random_prefix(
        //     &mut rng,
        //     &mut skcd_to_wire_ref_converter,
        //     "constant",
        // );
        // TODO should they instead be Gates??? or both Gate+Wire
        // If we only add Wires? how are we supposed to "set" them? They CAN NOT be "free floating"??
        // Or can they?
        // MAYBE use eg inputs[0] instead?
        let first_input = skcd
            .inputs
            .first()
            .ok_or_else(|| CircuitParserError::ConstantSpecialCaseMissingInput)?;
        let wire_constant = skcd_to_wire_ref_converter
            .get(first_input)
            .ok_or_else(|| CircuitParserError::InvalidGateId {
                gate_id: first_input.to_string(),
            })?
            .clone();

        // TODO(interstellar) how should we use skcd's a/b/go?
        let mut gates = Vec::<Gate>::with_capacity(skcd.gates.len());
        let mut outputs_start_end_indexes = (usize::MAX, usize::MIN);
        let mut max_gate_id = usize::MIN;
        // TODO constant_gate
        for skcd_gate in skcd.gates {
            // But `output` MUST always be set; this is what we use as Gate ID
            skcd_to_wire_ref_converter.insert(&skcd_gate.o);

            // **IMPORTANT** NOT ALL Gate to be built require x_ref and y_ref
            // so DO NOT unwrap here!
            // That would break the circuit building process!
            let mut x_ref = skcd_to_wire_ref_converter.get(&skcd_gate.a);
            let y_ref = skcd_to_wire_ref_converter.get(&skcd_gate.b);

            // [constant gate special case]
            match skcd_gate.r#type {
                // == interstellarpbskcd::SkcdGateType::Zero
                // == interstellarpbskcd::SkcdGateType::One
                0 | 1 => {
                    x_ref = Some(&wire_constant);
                }
                // Not a special case; it will be handled by `Gate::new_from_skcd_gate_type`
                _ => {}
            }

            let output_wire_ref =
                skcd_to_wire_ref_converter
                    .get(&skcd_gate.o)
                    .ok_or_else(|| CircuitParserError::OutputInvalidGateId {
                        gate_id: skcd_gate.o.clone(),
                    })?;

            gates.push(Gate::new_from_skcd_gate_type(
                skcd_gate.r#type,
                output_wire_ref,
                x_ref,
                y_ref,
            )?);

            // if gate is a circuit output wire then update the min/max indices
            if outputs_set.contains(&output_wire_ref) {
                outputs_start_end_indexes.0 = outputs_start_end_indexes.0.min(output_wire_ref.id);
                outputs_start_end_indexes.1 = outputs_start_end_indexes.1.max(output_wire_ref.id);
            }
            max_gate_id = max_gate_id.max(output_wire_ref.id);
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

        // compute stats etc
        let mut metadata = CircuitMetadata::new(outputs_start_end_indexes, max_gate_id);
        for gate in &gates {
            match gate.get_type() {
                crate::circuit::GateType::Binary {
                    gate_type,
                    input_a: _,
                    input_b: _,
                } => metadata.increment_binary_gate(
                    gate_type
                        .as_ref()
                        .ok_or_else(|| CircuitParserError::InvalidStateGateTypeNotSet)?,
                ),
                crate::circuit::GateType::Unary {
                    gate_type,
                    input_a: _,
                } => metadata.increment_unary_gate(
                    gate_type
                        .as_ref()
                        .ok_or_else(|| CircuitParserError::InvalidStateGateTypeNotSet)?,
                ),
                crate::circuit::GateType::Constant { value: _ } => {}
            }
        }

        log::info!("circuit metadata: {:?}", metadata);

        Ok(Circuit {
            circuit: CircuitInternal {
                inputs,
                outputs,
                gates,
                wires: skcd_to_wire_ref_converter.get_all_wires(),
            },
            config,
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::circuit::Circuit;
    use crate::tests::{FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS, FULL_ADDER_2BITS_ALL_INPUTS};

    #[test]
    fn test_eval_plain_full_adder_2bits() {
        let circ =
            Circuit::parse_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin")).unwrap();

        assert!(circ.num_evaluator_inputs() == 3);
        for (i, inputs) in FULL_ADDER_2BITS_ALL_INPUTS.iter().enumerate() {
            let outputs = circ.eval_plain(inputs).unwrap();
            assert_eq!(outputs, FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS[i]);
        }
    }
}
