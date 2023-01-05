use crate::InterstellarGarbledCircuit;
use rmp_serde::Deserializer;
use serde::{Deserialize, Serialize};

impl InterstellarGarbledCircuit {
    /// Serialize to msgpack format using RMP(Rust MSGPACK)
    pub fn serialize_msgpack(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.serialize(&mut rmp_serde::Serializer::new(&mut buf))
            .unwrap();

        buf
    }

    pub fn deserialize_msgpack(buf: &[u8]) -> Self {
        let mut de = Deserializer::new(buf);
        let actual: Self = Deserialize::deserialize(&mut de).unwrap();

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

        let buf = ref_garb.serialize_msgpack();
        let new_garb = InterstellarGarbledCircuit::deserialize_msgpack(&buf);

        assert_eq!(ref_garb, new_garb);
    }
}
