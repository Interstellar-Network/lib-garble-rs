use core::mem::size_of;

use bitvec::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    constant::{KAPPA, KAPPA_FACTOR},
    wire_value::WireValue,
};

// TODO u128? would it be faster?
type BitsInternal = u64;
/// The number of Bytes needed to store `MyBitArrayL`/`BlockL`
/// Typically this would be 8 b/c we are using `u64` internally for `bitvec`
pub(super) const KAPPA_BYTES: usize = KAPPA / size_of::<BitsInternal>();

type MyBitArrayL = BitArr!(for KAPPA, in BitsInternal, Lsb0);
type MyBitArrayP = BitArr!(for KAPPA * KAPPA_FACTOR, in BitsInternal, Lsb0);

/// The "external" Block,
/// "a random string of length l" (l <=> KAPPA)
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub(crate) struct BlockL {
    bits: MyBitArrayL,
}

/// The "internal" Block,
/// "a random string of length l'" (l' <=> 8 * l <=> 8 * KAPPA)
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
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

        // println!("new_projection : {res:?}");
        res
    }
}

impl From<BlockP> for BlockL {
    /// Truncate a `BlockP` into a `BlockL`
    // TODO is this needed? is there a better way to get L0/L1 from Delta and CompressedSet?
    fn from(block_p: BlockP) -> Self {
        let mut bits_l_array = MyBitArrayL::ZERO;
        bits_l_array.copy_from_bitslice(&block_p.bits.as_bitslice()[0..KAPPA_BYTES * KAPPA_FACTOR]);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projection_zero_with_one() {
        let zero = BlockP::new_zero();
        let one = BlockP::new_with2([u8::MAX; KAPPA_BYTES * KAPPA_FACTOR]);

        let result = BlockP::new_projection(&zero, &one);

        assert_eq!(result, zero);
    }

    #[test]
    fn test_projection_one_with_zero() {
        let zero = BlockP::new_zero();
        let one = BlockP::new_with2([u8::MAX; KAPPA_BYTES * KAPPA_FACTOR]);

        let result = BlockP::new_projection(&one, &zero);

        assert_eq!(result, zero);
    }

    #[test]
    fn test_projection_one_with_one() {
        let zero = BlockP::new_zero();
        let one = BlockP::new_with2([u8::MAX; KAPPA_BYTES * KAPPA_FACTOR]);

        let result = BlockP::new_projection(&one, &one);

        assert_eq!(result, one);
    }

    #[test]
    fn test_projection_one_with_test() {
        let zero = BlockP::new_zero();
        let one = BlockP::new_with2([u8::MAX; KAPPA_BYTES * KAPPA_FACTOR]);
        // NOTE: generated on Rust Playground
        let test = BlockP::new_with2([
            24, 87, 54, 170, 45, 105, 64, 249, 2, 110, 96, 207, 237, 118, 7, 70, 179, 188, 68, 6,
            107, 131, 120, 98, 33, 224, 122, 71, 252, 149, 106, 115, 142, 79, 61, 213, 30, 114, 82,
            182, 55, 61, 34, 134, 99, 45, 153, 21, 251, 73, 55, 201, 18, 140, 179, 164, 112, 73,
            80, 223, 218, 98, 195, 211, 25, 116, 173, 66, 124, 186, 182, 187, 7, 165, 125, 120,
            103, 46, 146, 73, 201, 197, 16, 172, 231, 30, 114, 222, 195, 124, 208, 183, 134, 248,
            84, 76, 167, 157, 108, 122, 16, 63, 219, 243, 145, 72, 157, 21, 35, 161, 16, 90, 213,
            214, 122, 31, 102, 49, 177, 149, 177, 73, 145, 69, 212, 121, 234, 151,
        ]);

        let result = BlockP::new_projection(&one, &test);

        assert_eq!(result, test);
    }

    #[test]
    fn test_projection_test_with_one() {
        let zero = BlockP::new_zero();
        let one = BlockP::new_with2([u8::MAX; KAPPA_BYTES * KAPPA_FACTOR]);
        // NOTE: generated on Rust Playground
        let test = BlockP::new_with2([
            24, 87, 54, 170, 45, 105, 64, 249, 2, 110, 96, 207, 237, 118, 7, 70, 179, 188, 68, 6,
            107, 131, 120, 98, 33, 224, 122, 71, 252, 149, 106, 115, 142, 79, 61, 213, 30, 114, 82,
            182, 55, 61, 34, 134, 99, 45, 153, 21, 251, 73, 55, 201, 18, 140, 179, 164, 112, 73,
            80, 223, 218, 98, 195, 211, 25, 116, 173, 66, 124, 186, 182, 187, 7, 165, 125, 120,
            103, 46, 146, 73, 201, 197, 16, 172, 231, 30, 114, 222, 195, 124, 208, 183, 134, 248,
            84, 76, 167, 157, 108, 122, 16, 63, 219, 243, 145, 72, 157, 21, 35, 161, 16, 90, 213,
            214, 122, 31, 102, 49, 177, 149, 177, 73, 145, 69, 212, 121, 234, 151,
        ]);

        let result = BlockP::new_projection(&test, &one);

        assert_eq!(result, test);
    }

    #[test]
    fn test_projection_test_with_test() {
        let zero = BlockP::new_zero();
        let one = BlockP::new_with2([u8::MAX; KAPPA_BYTES * KAPPA_FACTOR]);
        // NOTE: generated on Rust Playground
        let test = BlockP::new_with2([
            24, 87, 54, 170, 45, 105, 64, 249, 2, 110, 96, 207, 237, 118, 7, 70, 179, 188, 68, 6,
            107, 131, 120, 98, 33, 224, 122, 71, 252, 149, 106, 115, 142, 79, 61, 213, 30, 114, 82,
            182, 55, 61, 34, 134, 99, 45, 153, 21, 251, 73, 55, 201, 18, 140, 179, 164, 112, 73,
            80, 223, 218, 98, 195, 211, 25, 116, 173, 66, 124, 186, 182, 187, 7, 165, 125, 120,
            103, 46, 146, 73, 201, 197, 16, 172, 231, 30, 114, 222, 195, 124, 208, 183, 134, 248,
            84, 76, 167, 157, 108, 122, 16, 63, 219, 243, 145, 72, 157, 21, 35, 161, 16, 90, 213,
            214, 122, 31, 102, 49, 177, 149, 177, 73, 145, 69, 212, 121, 234, 151,
        ]);

        let result = BlockP::new_projection(&test, &test);

        assert_eq!(result, test);
    }

    #[test]
    fn test_projection_different() {
        let zero = BlockP::new_zero();
        let one = BlockP::new_with2([u8::MAX; KAPPA_BYTES * KAPPA_FACTOR]);
        // NOTE: generated on Rust Playground
        let test1 = BlockP::new_with2([
            24, 87, 54, 170, 45, 105, 64, 249, 2, 110, 96, 207, 237, 118, 7, 70, 179, 188, 68, 6,
            107, 131, 120, 98, 33, 224, 122, 71, 252, 149, 106, 115, 142, 79, 61, 213, 30, 114, 82,
            182, 55, 61, 34, 134, 99, 45, 153, 21, 251, 73, 55, 201, 18, 140, 179, 164, 112, 73,
            80, 223, 218, 98, 195, 211, 25, 116, 173, 66, 124, 186, 182, 187, 7, 165, 125, 120,
            103, 46, 146, 73, 201, 197, 16, 172, 231, 30, 114, 222, 195, 124, 208, 183, 134, 248,
            84, 76, 167, 157, 108, 122, 16, 63, 219, 243, 145, 72, 157, 21, 35, 161, 16, 90, 213,
            214, 122, 31, 102, 49, 177, 149, 177, 73, 145, 69, 212, 121, 234, 151,
        ]);
        let test2 = BlockP::new_with2([
            110, 241, 84, 253, 133, 26, 228, 117, 158, 135, 223, 230, 191, 94, 210, 247, 40, 28,
            20, 215, 219, 254, 131, 225, 176, 223, 232, 214, 157, 52, 70, 217, 77, 249, 132, 3,
            196, 53, 35, 160, 164, 3, 149, 240, 227, 208, 245, 52, 72, 174, 25, 50, 253, 43, 213,
            98, 195, 180, 255, 157, 38, 54, 111, 81, 103, 44, 116, 88, 130, 152, 192, 82, 199, 206,
            67, 202, 27, 160, 70, 107, 142, 137, 14, 240, 109, 247, 202, 246, 105, 225, 21, 119,
            198, 99, 108, 121, 29, 206, 108, 18, 238, 11, 53, 174, 158, 82, 78, 73, 167, 92, 43,
            190, 225, 74, 220, 135, 6, 55, 87, 55, 99, 111, 26, 232, 31, 25, 169, 62,
        ]);

        let result1 = BlockP::new_projection(&test1, &one);
        let result2 = BlockP::new_projection(&test2, &one);

        assert_ne!(result1, result2);
    }
}
