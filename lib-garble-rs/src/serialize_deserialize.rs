/// Implement de/serialization using Postcard https://github.com/jamesmunns/postcard
/// Why not others?
/// - msgpack rust: NOT compatible with no_std(and therefore fail in SGX env)
///   "rmp" crate SHOULD work, but "rmp-serde" definitely DOES NOT...
/// - prost: COULD works OK but we must re-implement all (de)serialization manually instead
///   of being able to re-use the Swanky provided "serde1" feature.
///   WOULD also require to add a few getters to expose deltas/Block/etc
///   NOTE: works in no_std/sgx only when using pregenerated .rs
use crate::EncodedGarblerInputs;
use crate::InterstellarGarbledCircuit;
use alloc::vec::Vec;
use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};

// impl InterstellarGarbledCircuit {
//     /// Serialize to postcard format using https://github.com/jamesmunns/postcard
//     fn serialize_postcard(&self) -> Vec<u8> {
//         let output: Vec<u8> = to_allocvec(self).unwrap();
//         output
//     }

//     /// Deserialize from postcard format using https://github.com/jamesmunns/postcard
//     fn deserialize_postcard(buf: &[u8]) -> Self {
//         let actual: Self = from_bytes(buf).unwrap();

//         actual
//     }
// }

/// That is the "package" sent to the client for evaluation
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct EvaluableGarbledCircuit {
    garb: InterstellarGarbledCircuit,
    encoded_garbler_inputs: EncodedGarblerInputs,
}

/// Serialize
/// Our use case only requires a subset of the whole (de)serialization so no need to expose the whole module
// TODO modify the API: it should probably take non-encoded inputs(ie &[u16])
pub fn serialize_for_evaluator(
    garb: InterstellarGarbledCircuit,
    encoded_garbler_inputs: EncodedGarblerInputs,
) -> Vec<u8> {
    // TODO(interstellar)? but is this the correct time to CHECK?
    assert_eq!(
        garb.encoder.num_garbler_inputs(),
        encoded_garbler_inputs.wires.len(),
        "wrong encoded_garbler_inputs len!"
    );

    let eval_garb = EvaluableGarbledCircuit {
        garb,
        encoded_garbler_inputs,
    };

    let buf: Vec<u8> = to_allocvec(&eval_garb).unwrap();

    buf
}

/// Deserialize
/// Our use case only requires a subset of the whole (de)serialization so no need to expose the whole module
pub fn deserialize_for_evaluator(buf: &[u8]) -> (InterstellarGarbledCircuit, EncodedGarblerInputs) {
    let (garb, encoded_garbler_inputs): (InterstellarGarbledCircuit, EncodedGarblerInputs) =
        from_bytes(buf).unwrap();

    (garb, encoded_garbler_inputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::garble_skcd;

    /// test that specific(=postcard) (de)serialization works
    #[test]
    fn test_serialize_deserialize_full_adder_2bits() {
        let ref_garb = garble_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin"));
        let encoded_garbler_inputs = ref_garb.encode_garbler_inputs(&[]);

        let buf = serialize_for_evaluator(ref_garb.clone(), encoded_garbler_inputs);
        let (new_garb, _new_encoded_garbler_inputs) = deserialize_for_evaluator(&buf);

        assert_eq!(ref_garb, new_garb);
    }

    /// test that specific(=postcard) (de)serialization works with display_message_120x52_2digits
    /// NOTE: contrary to "generic circuits"(cf above) we HAVE set some "garbler_inputs" in the Encoder and those SHOULD NOT
    /// be serialized(cf test after) so we compare manually
    #[test]
    fn test_serialize_deserialize_display_message_120x52_2digits() {
        let ref_garb = garble_skcd(include_bytes!(
            "../examples/data/display_message_120x52_2digits.skcd.pb.bin"
        ));
        let garbler_inputs = vec![0; ref_garb.encoder.num_garbler_inputs()];
        let encoded_garbler_inputs = ref_garb.encode_garbler_inputs(&garbler_inputs);

        let buf = serialize_for_evaluator(ref_garb.clone(), encoded_garbler_inputs);
        let (new_garb, _new_encoded_garbler_inputs) = deserialize_for_evaluator(&buf);

        assert_eq!(ref_garb.garbled, new_garb.garbled);
        assert_eq!(
            ref_garb.encoder.num_evaluator_inputs(),
            new_garb.encoder.num_evaluator_inputs()
        );
        assert_eq!(ref_garb.config, new_garb.config);
    }

    /// test that the client DOES NOT have access to Encoder's garbler_inputs
    #[test]
    fn test_encoder_has_no_garbler_inputs_display_message_120x52_2digits() {
        let ref_garb = garble_skcd(include_bytes!(
            "../examples/data/display_message_120x52_2digits.skcd.pb.bin"
        ));
        let garbler_inputs = vec![0; ref_garb.encoder.num_garbler_inputs()];
        let encoded_garbler_inputs = ref_garb.encode_garbler_inputs(&garbler_inputs);

        let buf = serialize_for_evaluator(ref_garb.clone(), encoded_garbler_inputs);
        let (new_garb, _new_encoded_garbler_inputs) = deserialize_for_evaluator(&buf);

        assert_eq!(new_garb.encoder.num_garbler_inputs(), 0);
    }
}
