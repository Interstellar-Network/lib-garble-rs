use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use core::mem::size_of;

use bitvec::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    constant::{KAPPA, KAPPA_FACTOR},
    wire_value::WireValue,
    GarblerError,
};

// TODO u128? would it be faster?
pub(super) type BitsInternal = u64;

pub(super) type MyBitArrayL = [BitsInternal; KAPPA_NB_ELEMENTS];
type MyBitArrayP = [BitsInternal; KAPPA_NB_ELEMENTS * KAPPA_FACTOR];

/// The number of Bytes needed to store `MyBitArrayL`/`BlockL`
/// Typically this would be 8 b/c we are using `u64` internally for `bitvec`
/// eg KAPPA = 128 and sizeof(u64) = 8 => `KAPPA_BYTES` = 128 / 8 => 16
// pub(super) const KAPPA_BYTES: usize = size_of::<MyBitArrayL>();
/// That is the number of "internal element"(eg `BitsInternal` = u64) needed
/// to represent a `MyBitArrayL`
/// eg KAPPA = 128 bits  //  `BitsInternal` = u64 = 64 bits => 128 / 64 => 2 elements
pub(super) const KAPPA_NB_ELEMENTS: usize = KAPPA / BitsInternal::BITS as usize;

/// The "external" Block,
/// "a random string of length l" (l <=> KAPPA)
///
/// About `clippy::unsafe_derive_deserialize`: `unsafe` is NOT used for `new` or other
/// serialization-related functions so we just ignore the warning.
// TODO is using `clippy::unsafe_derive_deserialize` dangerous?
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Default, Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub(super) struct BlockL {
    bits_words: MyBitArrayL,
}

/// The "internal" Block,
/// "a random string of length l'" (l' <=> 8 * l <=> 8 * KAPPA)
#[derive(PartialEq, Debug, Clone)]
pub(super) struct BlockP {
    bits_words: MyBitArrayP,
    // TODO?
    // bits_arr: [BlockL; KAPPA_FACTOR],
}

impl BlockL {
    // TODO should it instead be refactored into "new_random()"+moved to RandomOracle
    pub(super) fn new_with(initial_value: MyBitArrayL) -> Self {
        Self {
            bits_words: initial_value,
        }
    }

    pub(super) fn as_bytes(&self) -> &[u8] {
        // let slice: &[BitsInternal] = self.bits.as_raw_slice();
        // let ptr = slice.as_ptr() as *const u8;
        // let len = slice.len() * std::mem::size_of::<BitsInternal>();
        // unsafe { std::slice::from_raw_parts(ptr, len) }
        //

        // [
        //     self.bits_words[0].to_be_bytes(),
        //     self.bits_words[1].to_be_bytes(),
        // ]
        // .concat()
        // .as_slice()
        // let bits = self.bits_words.view_bits::<Lsb0>();
        // let bytes = bits.as_raw_slice();
        // bytes

        let ptr = self.bits_words.as_ptr().cast::<u8>();
        let len = self.bits_words.len() * size_of::<u64>();
        unsafe { alloc::slice::from_raw_parts(ptr, len) }
    }

    #[allow(dead_code)]
    pub(super) fn xor(&self, other: &BlockL) -> BlockL {
        let bits_words: Vec<BitsInternal> = self
            .bits_words
            .iter()
            .zip(other.bits_words.iter())
            .map(|(left, right)| left ^ right)
            .collect();

        Self {
            bits_words: unsafe { bits_words.try_into().unwrap_unchecked() },
        }
    }

    /// "A ◦ B = projection of A[i] for positions with B[i] = 1"
    pub(super) fn new_projection(left: &BlockL, right: &BlockL) -> Self {
        Self {
            bits_words: [
                left.bits_words[0] & right.bits_words[0],
                left.bits_words[1] & right.bits_words[1],
            ],
        }
    }
}

impl BlockP {
    /// Crate a new instance with the given value
    /// NOTE: Called by `random_oracle_g` so the input is (pseudo) random,
    /// so using `to_be_bytes` vs `to_le_bytes` does not really matter
    #[cfg(test)]
    pub(super) fn new_with2(initial_value: MyBitArrayP) -> Self {
        // TODO or use `from_be_bytes`? For the use case(which is creating new random blocks, it should not really matter)
        // let words: Vec<BitsInternal> = initial_value
        //     .chunks(size_of::<BitsInternal>())
        //     .map(|c| BitsInternal::from_le_bytes(c.try_into().unwrap()))
        //     .collect();
        // let words: [BitsInternal; KAPPA_NB_ELEMENTS * KAPPA_FACTOR] = words.try_into().unwrap();

        Self {
            bits_words: initial_value,
        }
    }

    /// Crate a new instance with the given value
    /// NOTE: Called by `random_oracle_g` so the input is (pseudo) random,
    /// so using `to_be_bytes` vs `to_le_bytes` does not really matter
    pub(super) fn new_with_raw_bytes(
        initial_value: [u8; KAPPA_NB_ELEMENTS * KAPPA_FACTOR * size_of::<BitsInternal>()],
    ) -> Self {
        // TODO or use `from_be_bytes`? For the use case(which is creating new random blocks, it should not really matter)
        let words: Vec<BitsInternal> = initial_value
            .chunks(size_of::<BitsInternal>())
            .map(|c| BitsInternal::from_le_bytes(unsafe { c.try_into().unwrap_unchecked() }))
            .collect();
        // let words: [BitsInternal; KAPPA_NB_ELEMENTS * KAPPA_FACTOR] = words.try_into().unwrap();

        Self {
            bits_words: unsafe { words.try_into().unwrap_unchecked() },
        }
    }

    pub(super) fn new_zero() -> Self {
        Self {
            bits_words: [0; KAPPA_NB_ELEMENTS * KAPPA_FACTOR],
        }
    }

    /// It REALLY important that `get_bit` and `set_bit` use exactly the same
    /// order, endianness, etc
    fn get_bits_internal_mut(&mut self) -> &mut BitSlice<u64> {
        self.bits_words.view_bits_mut::<Lsb0>()
    }

    fn get_bits_internal(&self) -> &BitSlice<u64> {
        self.bits_words.view_bits::<Lsb0>()
    }

    pub(super) fn get_bit(&self, index: usize) -> Result<WireValue, GarblerError> {
        let self_bits = self.get_bits_internal();

        if index >= self_bits.len() {
            return Err(GarblerError::BlockPBitOutOfRange { index });
        }

        unsafe {
            Ok(self_bits
                .get(index)
                .unwrap_unchecked()
                .as_ref()
                .to_owned()
                .into())
        }
    }

    /// Set the `index` to `true`
    pub(super) fn set_bit(&mut self, index: usize) {
        self.get_bits_internal_mut().set(index, true);
    }

    /// "A ◦ B = projection of A[i] for positions with B[i] = 1"
    pub(super) fn new_projection(left: &BlockP, right: &BlockP) -> Self {
        let bits_words: Vec<BitsInternal> = left
            .bits_words
            .iter()
            .zip(right.bits_words.iter())
            .map(|(left, right)| left & right)
            .collect();

        Self {
            bits_words: unsafe { bits_words.try_into().unwrap_unchecked() },
        }
    }
}

impl From<BlockP> for BlockL {
    /// Truncate a `BlockP` into a `BlockL`
    // TODO is this needed? is there a better way to get L0/L1 from Delta and CompressedSet?
    fn from(block_p: BlockP) -> Self {
        // let mut bits_l_array = MyBitArrayL::ZERO;
        // bits_l_array.copy_from_bitslice(&block_p.bits.as_bitslice()[0..KAPPA_BYTES * KAPPA_FACTOR]);
        Self {
            bits_words: unsafe {
                block_p
                    .bits_words
                    .split_at(KAPPA_NB_ELEMENTS)
                    .0
                    .try_into()
                    .unwrap_unchecked()
            },
        }
    }
}

impl From<&BlockP> for BlockL {
    /// Truncate a `BlockP` into a `BlockL`
    // TODO is this needed? is there a better way to get L0/L1 from Delta and CompressedSet?
    fn from(block_p: &BlockP) -> Self {
        // let mut bits_l_array = MyBitArrayL::ZERO;
        // bits_l_array.copy_from_bitslice(&block_p.bits.as_bitslice()[0..KAPPA_BYTES * KAPPA_FACTOR]);
        Self {
            bits_words: unsafe {
                block_p
                    .bits_words
                    .split_at(KAPPA_NB_ELEMENTS)
                    .0
                    .try_into()
                    .unwrap_unchecked()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_blocks() -> (BlockP, BlockP, BlockP, BlockP) {
        let zero = BlockP::new_zero();
        let one = BlockP::new_with2([u64::MAX; KAPPA_NB_ELEMENTS * KAPPA_FACTOR]);
        // NOTE: generated on Rust Playground
        //
        // use rand::{Rng, thread_rng};
        //
        // fn main() {
        //     let mut rng = thread_rng();
        //     let mut random_numbers: Vec<u64> = Vec::new();
        //
        //     for _ in 0..16 {
        //         let random_number = rng.gen();
        //         random_numbers.push(random_number);
        //     }
        //
        //     println!("Random numbers: {:?}", random_numbers);
        // }

        let test1 = BlockP::new_with2([
            3_951_001_893_725_728_678,
            17_561_894_908_598_795_415,
            3_273_299_927_427_316_065,
            4_016_781_436_536_637_665,
            3_759_867_147_464_905_433,
            4_273_494_230_197_193_221,
            3_529_531_907_751_757_055,
            16_273_736_933_959_562_170,
            16_977_210_453_145_070_413,
            4_260_534_243_702_315_869,
            8_876_721_923_944_456_293,
            6_706_553_457_839_696_430,
            11_459_371_310_689_979_744,
            17_420_813_315_993_560_429,
            16_645_214_173_008_843_092,
            1_335_969_637_496_639_684,
        ]);
        // NOTE: generated on Rust Playground
        let test2 = BlockP::new_with2([
            9_449_436_712_766_709_104,
            3_648_953_883_981_184_573,
            14_898_637_992_720_905_965,
            17_363_463_440_617_121_051,
            7_750_060_861_933_093_186,
            14_007_631_929_040_371_275,
            5_938_564_052_276_943_847,
            10_629_746_254_474_597_517,
            3_232_167_171_266_494_280,
            4_891_434_532_817_971_135,
            14_814_410_512_354_217_645,
            16_902_468_201_008_627_571,
            15_996_213_338_535_303_994,
            2_018_280_331_266_639_914,
            3_514_537_016_880_298_159,
            17_460_098_548_274_586_993,
        ]);

        (zero, one, test1, test2)
    }

    #[test]
    fn test_projection_zero_with_one() {
        let (zero, one, _test1, _test2) = get_test_blocks();

        let result = BlockP::new_projection(&zero, &one);

        assert_eq!(result, zero);
    }

    #[test]
    fn test_projection_one_with_zero() {
        let (zero, one, _test1, _test2) = get_test_blocks();

        let result = BlockP::new_projection(&one, &zero);

        assert_eq!(result, zero);
    }

    #[test]
    fn test_projection_one_with_one() {
        let (_zero, one, _test1, _test2) = get_test_blocks();

        let result = BlockP::new_projection(&one, &one);

        assert_eq!(result, one);
    }

    #[test]
    fn test_projection_one_with_test() {
        let (_zero, one, test1, _test2) = get_test_blocks();

        let result = BlockP::new_projection(&one, &test1);

        assert_eq!(result, test1);
    }

    #[test]
    fn test_projection_test_with_one() {
        let (_zero, one, test1, _test2) = get_test_blocks();

        let result = BlockP::new_projection(&test1, &one);

        assert_eq!(result, test1);
    }

    #[test]
    fn test_projection_test_with_test() {
        let (_zero, _one, test1, _test2) = get_test_blocks();

        let result = BlockP::new_projection(&test1, &test1);

        assert_eq!(result, test1);
    }

    #[test]
    fn test_projection_different() {
        let (_zero, one, test1, test2) = get_test_blocks();

        let result1 = BlockP::new_projection(&test1, &one);
        let result2 = BlockP::new_projection(&test2, &one);

        assert_ne!(result1, result2);
    }
}
