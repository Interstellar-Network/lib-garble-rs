use core::{mem::size_of, ops::BitAnd};

use bitvec::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    constant::{KAPPA, KAPPA_FACTOR},
    wire_value::WireValue,
};

// TODO u128? would it be faster?
pub(super) type BitsInternal = usize;

type MyBitArrayL = BitArr!(for KAPPA, in BitsInternal, Lsb0);
type MyBitArrayP = BitArr!(for KAPPA * KAPPA_FACTOR, in BitsInternal, Lsb0);

/// The number of Bytes needed to store `MyBitArrayL`/`BlockL`
/// Typically this would be 8 b/c we are using `u64` internally for `bitvec`
/// eg KAPPA = 128 and sizeof(u64) = 8 => KAPPA_BYTES = 128 / 8 => 16
pub(super) const KAPPA_BYTES: usize = size_of::<MyBitArrayL>();
/// That is the number of "internal element"(eg BitsInternal = u64) needed
/// to represent a `MyBitArrayL`
/// eg KAPPA = 128 + BitsInternal = u64 => 128 / 64 => 2 elements
pub(super) const KAPPA_NB_ELEMENTS: usize = size_of::<MyBitArrayL>() / size_of::<BitsInternal>();

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
    pub(super) fn new_with(initial_value: [BitsInternal; KAPPA_NB_ELEMENTS]) -> Self {
        Self {
            bits: MyBitArrayL::from(initial_value),
        }
    }

    pub(super) fn as_bytes(&self) -> &[u8] {
        let slice: &[BitsInternal] = self.bits.as_raw_slice();
        let ptr = slice.as_ptr() as *const u8;
        let len = slice.len() * std::mem::size_of::<BitsInternal>();
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

impl BlockP {
    pub(super) fn new_with2(initial_value: [u8; KAPPA_BYTES * KAPPA_FACTOR]) -> Self {
        // TODO or use `from_be_bytes`? For the use case(which is creating new random blocks, it should not really matter)
        let words: Vec<BitsInternal> = initial_value
            .chunks(size_of::<BitsInternal>())
            .map(|c| BitsInternal::from_le_bytes(c.try_into().unwrap()))
            .collect();
        let words: [BitsInternal; KAPPA_NB_ELEMENTS * KAPPA_FACTOR] = words.try_into().unwrap();

        Self {
            bits: MyBitArrayP::from(words),
        }
    }

    pub(super) fn new_zero() -> Self {
        Self {
            bits: MyBitArrayP::ZERO,
        }
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
        Self {
            bits: left.bits.bitand(right.bits),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_blocks() -> (BlockP, BlockP, BlockP, BlockP) {
        let zero = BlockP::new_zero();
        let one = BlockP::new_with2([u8::MAX; KAPPA_BYTES * KAPPA_FACTOR]);
        // NOTE: generated on Rust Playground
        let test1 = BlockP::new_with2([
            243, 108, 244, 60, 108, 187, 206, 32, 89, 240, 106, 139, 37, 186, 147, 52, 7, 147, 74,
            93, 45, 65, 40, 141, 2, 37, 250, 215, 246, 210, 54, 193, 250, 169, 180, 2, 8, 244, 170,
            44, 13, 230, 176, 90, 162, 170, 133, 176, 159, 217, 148, 70, 26, 102, 143, 136, 22,
            168, 55, 25, 211, 59, 139, 22, 21, 101, 144, 36, 211, 181, 31, 144, 26, 190, 175, 134,
            213, 61, 203, 50, 163, 249, 206, 131, 132, 174, 204, 171, 65, 237, 4, 244, 101, 98, 85,
            232, 81, 138, 85, 195, 66, 108, 142, 8, 11, 57, 10, 243, 162, 216, 208, 217, 218, 235,
            168, 214, 229, 92, 46, 251, 153, 52, 242, 198, 26, 34, 27, 70,
        ]);
        // NOTE: generated on Rust Playground
        let test2 = BlockP::new_with2([
            247, 165, 155, 149, 68, 116, 58, 1, 2, 23, 18, 177, 131, 152, 56, 13, 128, 5, 85, 45,
            176, 128, 41, 247, 35, 166, 4, 69, 68, 70, 153, 52, 195, 77, 70, 113, 79, 92, 247, 52,
            156, 188, 83, 229, 253, 240, 225, 224, 219, 158, 175, 106, 119, 226, 241, 199, 150,
            155, 104, 196, 233, 246, 118, 180, 206, 193, 213, 90, 137, 158, 243, 51, 101, 182, 17,
            42, 84, 120, 207, 32, 157, 19, 18, 170, 24, 192, 203, 82, 175, 34, 217, 215, 174, 90,
            216, 233, 73, 171, 246, 157, 17, 129, 81, 51, 141, 65, 216, 252, 51, 98, 239, 179, 97,
            248, 251, 200, 45, 102, 63, 111, 243, 77, 161, 4, 220, 112, 203, 93,
        ]);

        (zero, one, test1, test2)
    }

    #[test]
    fn test_projection_zero_with_one() {
        let (zero, one, test1, test2) = get_test_blocks();

        let result = BlockP::new_projection(&zero, &one);

        assert_eq!(result, zero);
    }

    #[test]
    fn test_projection_one_with_zero() {
        let (zero, one, test1, test2) = get_test_blocks();

        let result = BlockP::new_projection(&one, &zero);

        assert_eq!(result, zero);
    }

    #[test]
    fn test_projection_one_with_one() {
        let (zero, one, test1, test2) = get_test_blocks();

        let result = BlockP::new_projection(&one, &one);

        assert_eq!(result, one);
    }

    #[test]
    fn test_projection_one_with_test() {
        let (zero, one, test1, test2) = get_test_blocks();

        let result = BlockP::new_projection(&one, &test1);

        assert_eq!(result, test1);
    }

    #[test]
    fn test_projection_test_with_one() {
        let (zero, one, test1, test2) = get_test_blocks();

        let result = BlockP::new_projection(&test1, &one);

        assert_eq!(result, test1);
    }

    #[test]
    fn test_projection_test_with_test() {
        let (zero, one, test1, test2) = get_test_blocks();

        let result = BlockP::new_projection(&test1, &test1);

        assert_eq!(result, test1);
    }

    #[test]
    fn test_projection_different() {
        let (zero, one, test1, test2) = get_test_blocks();

        let result1 = BlockP::new_projection(&test1, &one);
        let result2 = BlockP::new_projection(&test2, &one);

        assert_ne!(result1, result2);
    }
}
