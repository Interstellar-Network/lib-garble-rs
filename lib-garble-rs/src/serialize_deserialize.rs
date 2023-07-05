/// Implement de/serialization using Postcard <https://github.com/jamesmunns/postcard>
/// Why not others?
/// - msgpack rust: NOT compatible with `no_std`(and therefore fail in SGX env)
///   "rmp" crate SHOULD work, but "rmp-serde" definitely DOES NOT...
/// - prost: COULD works OK but we must re-implement all (de)serialization manually instead
///   of being able to re-use the Swanky provided "serde1" feature.
///   WOULD also require to add a few getters to expose deltas/Block/etc
///   NOTE: works in `no_std/sgx` only when using pregenerated .rs
use crate::EncodedGarblerInputs;
use crate::GarbledCircuit;
use alloc::vec::Vec;
use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

/// That is the "package" sent to the client for evaluation
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct EvaluableGarbledCircuit {
    garb: GarbledCircuit,
    encoded_garbler_inputs: EncodedGarblerInputs,
}

#[derive(Debug, Snafu)]
pub enum Error {
    SerializerDeserializerInternalError {
        err: postcard::Error,
    },
    /// "wrong encoded_garbler_inputs len!"
    SerializeForEvaluatorWrongInputsLength {
        inputs_len: usize,
        expected_len: usize,
    },
}

/// Serialize
/// Our use case only requires a subset of the whole (de)serialization so no need to expose the whole module
///# Errors
///
/// `postcard::Error` if the serialization failed
///
// TODO modify the API: it should probably take non-encoded inputs(ie &[u16])
pub fn serialize_for_evaluator(
    garb: GarbledCircuit,
    encoded_garbler_inputs: EncodedGarblerInputs,
) -> Result<Vec<u8>, Error> {
    if garb.num_garbler_inputs() as usize != encoded_garbler_inputs.encoded.len() {
        return Err(Error::SerializeForEvaluatorWrongInputsLength {
            inputs_len: encoded_garbler_inputs.encoded.len(),
            expected_len: garb.num_garbler_inputs() as usize,
        });
    }

    let eval_garb = EvaluableGarbledCircuit {
        garb,
        encoded_garbler_inputs,
    };

    let buf: Vec<u8> = to_allocvec(&eval_garb)
        .map_err(|err| Error::SerializerDeserializerInternalError { err })?;

    Ok(buf)
}

/// Deserialize
/// Our use case only requires a subset of the whole (de)serialization so no need to expose the whole module
///
/// # Errors
///
/// `postcard::Error` if the deserialization failed
///
pub fn deserialize_for_evaluator(
    buf: &[u8],
) -> Result<(GarbledCircuit, EncodedGarblerInputs), postcard::Error> {
    let (garb, encoded_garbler_inputs): (GarbledCircuit, EncodedGarblerInputs) = from_bytes(buf)?;

    Ok((garb, encoded_garbler_inputs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        garble_skcd, garble_skcd_with_seed, garbled_display_circuit_prepare_garbler_inputs,
    };

    /// test that specific(=postcard) (de)serialization works
    #[test]
    fn test_serialize_deserialize_full_adder_2bits() {
        let mut ref_garb =
            garble_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin")).unwrap();
        let encoded_garbler_inputs = ref_garb.encode_garbler_inputs(&[]);

        let buf = serialize_for_evaluator(ref_garb.clone(), encoded_garbler_inputs).unwrap();
        let (new_garb, _new_encoded_garbler_inputs) = deserialize_for_evaluator(&buf).unwrap();

        garbled_circuit_reset_gate_type(&mut ref_garb);

        assert_eq!(ref_garb, new_garb);
    }

    /// test that specific(=postcard) (de)serialization works with display_message_120x52_2digits
    /// NOTE: contrary to "generic circuits"(cf above) we HAVE set some "garbler_inputs" in the Encoder and those SHOULD NOT
    /// be serialized(cf test after) so we compare manually
    #[test]
    fn test_serialize_deserialize_display_message_120x52_2digits() {
        let mut ref_garb = garble_skcd(include_bytes!(
            "../examples/data/display_message_120x52_2digits.skcd.pb.bin"
        ))
        .unwrap();
        let garbler_inputs = vec![0; ref_garb.num_garbler_inputs() as usize];
        let encoded_garbler_inputs = ref_garb.encode_garbler_inputs(&garbler_inputs);

        let buf = serialize_for_evaluator(ref_garb.clone(), encoded_garbler_inputs).unwrap();
        let (new_garb, _new_encoded_garbler_inputs) = deserialize_for_evaluator(&buf).unwrap();

        garbled_circuit_reset_gate_type(&mut ref_garb);

        assert_eq!(ref_garb, new_garb);
        assert_eq!(
            ref_garb.num_evaluator_inputs(),
            new_garb.num_evaluator_inputs()
        );
        assert_eq!(ref_garb.config, new_garb.config);
    }

    #[test]
    fn test_serialize_golden_display_message_120x52_2digits() {
        let ref_garb = garble_skcd_with_seed(
            include_bytes!("../examples/data/display_message_120x52_2digits.skcd.pb.bin"),
            424242,
        )
        .unwrap();
        let garbler_inputs = vec![4, 2];
        let encoded_garbler_inputs = garbled_display_circuit_prepare_garbler_inputs(
            &ref_garb,
            &garbler_inputs,
            "test message",
        )
        .unwrap();

        let buf = serialize_for_evaluator(ref_garb.clone(), encoded_garbler_inputs).unwrap();

        let ref_buf =
            include_bytes!("../examples/data/display_message_120x52_2digits.garbled.pb.bin");

        assert_eq!(buf, ref_buf, "failed {:#?} vs {:#?}", buf, ref_buf);
    }

    /// test that the client DOES NOT have access to Encoder's garbler_inputs
    #[test]
    // TODO(security) [security] we SHOULD NOT be able to call `encoding_internal` after `(de)serialize_for_evaluator`
    //  cf `InputEncodingSet` -> SHOULD probably be refactored(splitted) into "garbler" vs "evaluator"
    #[ignore]
    fn test_encoder_has_no_garbler_inputs_display_message_120x52_2digits() {
        let mut ref_garb = garble_skcd(include_bytes!(
            "../examples/data/display_message_120x52_2digits.skcd.pb.bin"
        ))
        .unwrap();
        let garbler_inputs = vec![0; ref_garb.num_garbler_inputs() as usize];
        let encoded_garbler_inputs = ref_garb.encode_garbler_inputs(&garbler_inputs);

        let buf = serialize_for_evaluator(ref_garb.clone(), encoded_garbler_inputs).unwrap();
        let (new_garb, _new_encoded_garbler_inputs) = deserialize_for_evaluator(&buf).unwrap();

        garbled_circuit_reset_gate_type(&mut ref_garb);

        assert_eq!(new_garb.num_garbler_inputs(), 0);
    }

    /// IMPORTANT: for security/privacy, we DO NOT serialize the GateType
    /// so we must clean the Gate
    fn garbled_circuit_reset_gate_type(garbled: &mut GarbledCircuit) {
        for gate in garbled.garbled.circuit.gates.iter_mut() {
            match &mut gate.internal {
                crate::circuit::GateType::Binary {
                    ref mut gate_type,
                    input_a,
                    input_b,
                } => *gate_type = None,
                crate::circuit::GateType::Unary {
                    ref mut gate_type,
                    input_a,
                } => *gate_type = None,
                crate::circuit::GateType::Constant { value } => {}
            }
        }
    }
}
