/// Implement de/serialization using Postcard <https://github.com/jamesmunns/postcard>
/// Why not others?
/// - msgpack rust: NOT compatible with `no_std`(and therefore fail in SGX env)
///   "rmp" crate SHOULD work, but "rmp-serde" definitely DOES NOT...
/// - prost: COULD works OK but we must re-implement all (de)serialization manually instead
///   of being able to re-use the Swanky provided "serde1" feature.
///   WOULD also require to add a few getters to expose deltas/Block/etc
///   NOTE: works in `no_std/sgx` only when using pregenerated .rs
use alloc::vec::Vec;

use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};

use crate::EncodedGarblerInputs;
use crate::GarbledCircuit;
use crate::InterstellarError;

/// That is the "package" sent to the client for evaluation
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct EvaluableGarbledCircuit {
    garb: GarbledCircuit,
    encoded_garbler_inputs: EncodedGarblerInputs,
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
) -> Result<Vec<u8>, InterstellarError> {
    // If display circuits: we check against `num_garbler_inputs`
    // else we check against `num_inputs`
    let expected_inputs_len = garb.num_inputs();
    if expected_inputs_len != encoded_garbler_inputs.encoded.len() {
        return Err(InterstellarError::SerializeForEvaluatorWrongInputsLength {
            inputs_len: encoded_garbler_inputs.encoded.len(),
            expected_len: expected_inputs_len,
        });
    }

    let eval_garb = EvaluableGarbledCircuit {
        garb,
        encoded_garbler_inputs,
    };

    let buf: Vec<u8> = to_allocvec(&eval_garb)
        .map_err(|err| InterstellarError::SerializerDeserializerInternalError { err })?;

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
) -> Result<(GarbledCircuit, EncodedGarblerInputs), InterstellarError> {
    let (garb, encoded_garbler_inputs): (GarbledCircuit, EncodedGarblerInputs) = from_bytes(buf)
        .map_err(|err| InterstellarError::SerializerDeserializerInternalError { err })?;

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
        let mut ref_garb = garble_skcd(include_bytes!(
            "../examples/data/result_abc_full_adder.postcard.bin"
        ))
        .unwrap();
        let encoded_garbler_inputs = ref_garb.encode_inputs(&[]);

        let buf = serialize_for_evaluator(ref_garb.clone(), encoded_garbler_inputs).unwrap();
        let (new_garb, _new_encoded_garbler_inputs) = deserialize_for_evaluator(&buf).unwrap();

        assert_eq!(ref_garb, new_garb);
    }

    /// test that specific(=postcard) (de)serialization works with `display_message_120x52_2digits`
    /// NOTE: contrary to "generic circuits"(cf above) we HAVE set some "`garbler_inputs`" in the Encoder and those SHOULD NOT
    /// be serialized(cf test after) so we compare manually
    #[test]
    fn test_serialize_deserialize_display_message_120x52_2digits() {
        let mut ref_garb = garble_skcd(include_bytes!(
            "../examples/data/result_display_message_120x52_2digits.postcard.bin"
        ))
        .unwrap();
        let garbler_inputs = vec![0; ref_garb.num_inputs() as usize];
        let encoded_garbler_inputs = ref_garb.encode_inputs(&garbler_inputs);

        let buf = serialize_for_evaluator(ref_garb.clone(), encoded_garbler_inputs).unwrap();
        let (new_garb, _new_encoded_garbler_inputs) = deserialize_for_evaluator(&buf).unwrap();

        assert_eq!(ref_garb.get_display_config(), new_garb.get_display_config());
        assert_eq!(ref_garb, new_garb);
        assert_eq!(
            ref_garb.num_evaluator_inputs(),
            new_garb.num_evaluator_inputs()
        );
    }

    #[test]
    fn test_serialize_golden_display_message_120x52_2digits() {
        let ref_garb = garble_skcd_with_seed(
            include_bytes!("../examples/data/result_display_message_120x52_2digits.postcard.bin"),
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

        let buf = serialize_for_evaluator(ref_garb, encoded_garbler_inputs).unwrap();

        let ref_buf =
            include_bytes!("../examples/data/display_message_120x52_2digits.garbled.pb.bin");

        assert_eq!(buf, ref_buf, "failed {buf:#?} vs {ref_buf:#?}");
    }

    /// test that the client DOES NOT have access to Encoder's `garbler_inputs`
    #[test]
    // TODO(security) [security] we SHOULD NOT be able to call `encoding_internal` after `(de)serialize_for_evaluator`
    //  cf `InputEncodingSet` -> SHOULD probably be refactored(splitted) into "garbler" vs "evaluator"
    #[ignore]
    fn test_encoder_has_no_garbler_inputs_display_message_120x52_2digits() {
        let mut ref_garb = garble_skcd(include_bytes!(
            "../examples/data/result_display_message_120x52_2digits.postcard.bin"
        ))
        .unwrap();
        let garbler_inputs = vec![0; ref_garb.num_inputs() as usize];
        let encoded_garbler_inputs = ref_garb.encode_inputs(&garbler_inputs);

        let buf = serialize_for_evaluator(ref_garb.clone(), encoded_garbler_inputs).unwrap();
        let (new_garb, _new_encoded_garbler_inputs) = deserialize_for_evaluator(&buf).unwrap();

        assert_eq!(new_garb.num_inputs(), 0);
    }
}
