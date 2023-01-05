use crate::InterstellarGarbledCircuit;
use alloc::vec::Vec;
use postcard::{from_bytes, to_allocvec};

/// Implement de/serialization using Postcard
/// Why not others?
/// - msgpack rust: NOT compatible with no_std(and therefore fail in SGX env)
///   "rmp" crate SHOULD work, but "rmp-serde" definitely DOES NOT...
impl InterstellarGarbledCircuit {
    /// Serialize to postcard format using https://github.com/jamesmunns/postcard
    pub fn serialize_postcard(&self) -> Vec<u8> {
        let output: Vec<u8> = to_allocvec(self).unwrap();
        output
    }

    /// Deserialize from postcard format using https://github.com/jamesmunns/postcard
    pub fn deserialize_postcard(buf: &[u8]) -> Self {
        let actual: Self = from_bytes(buf).unwrap();

        actual
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::garble_skcd;

    #[test]
    fn test_serialize_deserialize_full_adder_2bits() {
        let ref_garb = garble_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin"));

        let buf = ref_garb.serialize_postcard();
        let new_garb = InterstellarGarbledCircuit::deserialize_postcard(&buf);

        assert_eq!(ref_garb, new_garb);
    }
}
