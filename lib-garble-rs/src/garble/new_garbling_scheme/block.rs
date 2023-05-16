use core::mem::size_of;

use bitvec::prelude::*;

use super::{
    constant::{KAPPA, KAPPA_FACTOR},
    WireValue,
};

// TODO u128? would it be faster?
type BitsInternal = u64;
pub(super) const KAPPA_BYTES: usize = KAPPA / size_of::<BitsInternal>();

type MyBitArrayL = BitArr!(for KAPPA, in BitsInternal);
type MyBitArrayP = BitArr!(for KAPPA * KAPPA_FACTOR, in BitsInternal);

/// The "external" Block,
/// "a random string of length l" (l <=> KAPPA)
#[derive(PartialEq, Debug)]
pub(crate) struct BlockL {
    bits: MyBitArrayL,
}

/// The "internal" Block,
/// "a random string of length l'" (l' <=> 8 * l <=> 8 * KAPPA)
#[derive(PartialEq, Debug, Clone)]
pub(crate) struct BlockP {
    bits: MyBitArrayP,
}

impl BlockL {
    // TODO should it instead be refactored into "new_random()"+moved to RandomOracle
    pub(super) fn new_with(initial_value: [u64; 2]) -> Self {
        Self {
            bits: MyBitArrayL::from(initial_value),
        }
    }

    pub(super) fn new_with2(initial_value: [u8; KAPPA_BYTES]) -> Self {
        // TODO or use `from_be_bytes`? For the use case(which is creating new random blocks, it should not really matter)
        let words: Vec<u64> = initial_value
            .chunks(8)
            .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
            .collect();
        let words: [u64; 2] = words.try_into().unwrap();

        Self {
            bits: MyBitArrayL::from(words),
        }
    }

    pub(super) fn as_bytes(&self) -> &[u8] {
        let slice: &[u64] = self.bits.as_raw_slice();
        let ptr = slice.as_ptr() as *const u8;
        let len = slice.len() * std::mem::size_of::<u64>();
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

impl BlockP {
    pub(super) fn new_with2(initial_value: [u8; KAPPA_BYTES * KAPPA_FACTOR]) -> Self {
        // TODO or use `from_be_bytes`? For the use case(which is creating new random blocks, it should not really matter)
        let words: Vec<u64> = initial_value
            .chunks(8)
            .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
            .collect();
        let words: [u64; 2 * KAPPA_FACTOR] = words.try_into().unwrap();

        Self {
            bits: MyBitArrayP::from(words),
        }
    }

    pub(super) fn new_zero() -> Self {
        Self::new_with2([0; KAPPA_BYTES * KAPPA_FACTOR])
    }

    pub(super) fn get_bit(&self, index: usize) -> WireValue {
        self.bits
            .get(index)
            .expect("get_bit: outside of range?")
            .as_ref()
            .to_owned()
            .into()
    }

    /// Set the `index` to `true`
    pub(super) fn set_bit(&mut self, index: usize) {
        self.bits.set(index, true);
    }

    /// "A ◦ B = projection of A[i] for positions with B[i] = 1"
    pub(crate) fn new_projection(left: &BlockP, right: &BlockP) -> Self {
        let mut res = Self::new_zero();

        for (idx, bit) in right.bits.iter().enumerate() {
            if *bit {
                res.bits.set(idx, left.bits[idx]);
            }
        }

        res
    }
}

impl From<BlockP> for BlockL {
    /// Truncate a `BlockP` into a `BlockL`
    // TODO is this needed? is there a better way to get L0/L1 from Delta and CompressedSet?
    fn from(block_p: BlockP) -> Self {
        let mut bits_l_array = MyBitArrayL::ZERO;
        bits_l_array.copy_from_bitslice(block_p.bits.as_bitslice());
        Self { bits: bits_l_array }
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
