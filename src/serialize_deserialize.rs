use crate::InterstellarGarbledCircuit;
use alloc::vec::Vec;
use postcard::{from_bytes, to_allocvec};

pub trait MySerializable {
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(buf: &[u8]) -> Self;
}

/// Implement de/serialization using Postcard
/// Why not others?
/// - msgpack rust: NOT compatible with no_std(and therefore fail in SGX env)
///   "rmp" crate SHOULD work, but "rmp-serde" definitely DOES NOT...
/// - prost: COULD works OK but we must re-implement all (de)serialization manually instead
///   of being able to re-use the Swanky provided "serde1" feature.
///   WOULD also require to add a few getters to expose deltas/Block/etc
///   NOTE: works in no_std/sgx only when using pregenerated .rs
impl InterstellarGarbledCircuit {
    /// Serialize to postcard format using https://github.com/jamesmunns/postcard
    fn serialize_postcard(&self) -> Vec<u8> {
        let output: Vec<u8> = to_allocvec(self).unwrap();
        output
    }

    /// Deserialize from postcard format using https://github.com/jamesmunns/postcard
    fn deserialize_postcard(buf: &[u8]) -> Self {
        let actual: Self = from_bytes(buf).unwrap();

        actual
    }
}

impl MySerializable for InterstellarGarbledCircuit {
    fn serialize(&self) -> Vec<u8> {
        self.serialize_postcard()
    }

    fn deserialize(buf: &[u8]) -> Self {
        Self::deserialize_postcard(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::garble_skcd;

    /// test that specific(=postcard) (de)serialization works
    #[test]
    fn test_serialize_postcard_deserialize_full_adder_2bits() {
        let ref_garb = garble_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin"));

        let buf = ref_garb.serialize_postcard();
        let new_garb = InterstellarGarbledCircuit::deserialize_postcard(&buf);

        assert_eq!(ref_garb, new_garb);
    }

    /// test that specific(=postcard) (de)serialization works with display_message_120x52_2digits
    /// NOTE: contrary to "generic circuits"(cf above) we HAVE "garbler_inputs" in the encoder and those SHOULD NOT
    /// be serialized(cf test after) so we compare manually
    #[test]
    fn test_serialize_postcard_deserialize_display_message_120x52_2digits() {
        let ref_garb = garble_skcd(include_bytes!(
            "../examples/data/display_message_120x52_2digits.skcd.pb.bin"
        ));

        let buf = ref_garb.serialize_postcard();
        let new_garb = InterstellarGarbledCircuit::deserialize_postcard(&buf);

        assert_eq!(ref_garb.garbled, new_garb.garbled);
        assert_eq!(
            ref_garb.encoder.num_evaluator_inputs(),
            new_garb.encoder.num_evaluator_inputs()
        );
        assert_eq!(ref_garb.config, new_garb.config);
    }

    /// test that trait (de)serialization works
    #[test]
    fn test_serialize_deserialize_trait() {
        let ref_garb = garble_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin"));

        let buf = ref_garb.serialize();
        let new_garb = InterstellarGarbledCircuit::deserialize(&buf);

        assert_eq!(ref_garb, new_garb);
    }

    /// test that the client DOES NOT have access to Encoder's garbler_inputs
    #[test]
    fn test_encoder_has_no_garbler_inputs_display_message_120x52_2digits() {
        let ref_garb = garble_skcd(include_bytes!(
            "../examples/data/display_message_120x52_2digits.skcd.pb.bin"
        ));

        let buf = ref_garb.serialize();
        let new_garb = InterstellarGarbledCircuit::deserialize(&buf);

        assert_eq!(new_garb.encoder.num_garbler_inputs(), 0);
    }
}
