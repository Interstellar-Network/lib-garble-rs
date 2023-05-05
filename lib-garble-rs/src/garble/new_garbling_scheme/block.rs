use bitvec::prelude::*;

use crate::garble::new_garbling_scheme::constant::KAPPA;

// TODO u128? would it be faster?
type MyBitArray = BitArr!(for KAPPA, in u64);

#[derive(PartialEq, Debug)]
pub(crate) struct Block {
    bits: MyBitArray,
}

impl Block {
    // TODO should it instead be refactored into "new_random()"+moved to RandomOracle
    pub(crate) fn new_with(initial_value: [u64; 2]) -> Self {
        Self {
            bits: MyBitArray::from(initial_value),
        }
    }

    pub(crate) fn new_with2(initial_value: [u8; 16]) -> Self {
        // TODO or use `from_be_bytes`? For the use case(which is creating new random blocks, it should not really matter)
        let words: [u64; 2] = [
            u64::from_le_bytes(initial_value[0..8].try_into().unwrap()),
            u64::from_le_bytes(initial_value[8..16].try_into().unwrap()),
        ];

        Self {
            bits: MyBitArray::from(words),
        }
    }
}

// struct Block {
//     val: u128,
// }

// impl Block {
//     fn random() -> Self {
//         // TODO proper random; or better use Scuttlebutt directly
//         Block { val: 42 }
//     }
// }

// #[derive(PartialEq)]
// struct Label0 {
//     bits: LabelBits,
// }

// #[derive(PartialEq)]
// struct Label1 {
//     bits: LabelBits,
// }
