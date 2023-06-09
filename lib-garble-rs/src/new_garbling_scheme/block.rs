use core::{mem::size_of, ops::BitAnd};

use bitvec::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    constant::{KAPPA, KAPPA_FACTOR},
    wire_value::WireValue,
};

// TODO u128? would it be faster?
pub(super) type BitsInternal = u64;

type MyBitArrayL = [BitsInternal; KAPPA_NB_ELEMENTS];
type MyBitArrayP = [BitsInternal; KAPPA_NB_ELEMENTS * KAPPA_FACTOR];

/// The number of Bytes needed to store `MyBitArrayL`/`BlockL`
/// Typically this would be 8 b/c we are using `u64` internally for `bitvec`
/// eg KAPPA = 128 and sizeof(u64) = 8 => KAPPA_BYTES = 128 / 8 => 16
// pub(super) const KAPPA_BYTES: usize = size_of::<MyBitArrayL>();
/// That is the number of "internal element"(eg BitsInternal = u64) needed
/// to represent a `MyBitArrayL`
/// eg KAPPA = 128 bits  //  BitsInternal = u64 = 64 bits => 128 / 64 => 2 elements
pub(super) const KAPPA_NB_ELEMENTS: usize = KAPPA / (size_of::<BitsInternal>() * 8);

/// The "external" Block,
/// "a random string of length l" (l <=> KAPPA)
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub(crate) struct BlockL {
    bits_words: MyBitArrayL,
}

/// The "internal" Block,
/// "a random string of length l'" (l' <=> 8 * l <=> 8 * KAPPA)
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub(crate) struct BlockP {
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

        let ptr = self.bits_words.as_ptr() as *const u8;
        let len = self.bits_words.len() * size_of::<u64>();
        unsafe { alloc::slice::from_raw_parts(ptr, len) }
    }
}

impl BlockP {
    /// Crate a new instance with the given value
    /// NOTE: Called by `random_oracle_g` so the input is (pseudo) random,
    /// so using to_be_bytes vs to_le_bytes does not really matter
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
    /// so using to_be_bytes vs to_le_bytes does not really matter
    pub(super) fn new_with_raw_bytes(
        initial_value: [u8; KAPPA_NB_ELEMENTS * KAPPA_FACTOR * size_of::<BitsInternal>()],
    ) -> Self {
        // TODO or use `from_be_bytes`? For the use case(which is creating new random blocks, it should not really matter)
        let words: Vec<BitsInternal> = initial_value
            .chunks(size_of::<BitsInternal>())
            .map(|c| BitsInternal::from_le_bytes(c.try_into().unwrap()))
            .collect();
        // let words: [BitsInternal; KAPPA_NB_ELEMENTS * KAPPA_FACTOR] = words.try_into().unwrap();

        Self {
            bits_words: words.try_into().unwrap(),
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

    pub(super) fn get_bit(&self, index: usize) -> WireValue {
        let self_bits = self.get_bits_internal();

        self_bits
            .get(index)
            .expect("get_bit: outside of range?")
            .as_ref()
            .to_owned()
            .into()
    }

    /// Set the `index` to `true`
    pub(super) fn set_bit(&mut self, index: usize) {
        self.get_bits_internal_mut().set(index, true);
    }

    /// "A ◦ B = projection of A[i] for positions with B[i] = 1"
    pub(crate) fn new_projection(left: &BlockP, right: &BlockP) -> Self {
        // let bits_words = Self::new_zero();

        let bits_words: Vec<BitsInternal> = left
            .bits_words
            .iter()
            .zip(right.bits_words.iter())
            .map(|(left, right)| left & right)
            .collect();

        Self {
            bits_words: bits_words.try_into().unwrap(),
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
            bits_words: block_p
                .bits_words
                .split_at(KAPPA_NB_ELEMENTS)
                .0
                .try_into()
                .expect("BlockL::from slice with incorrect length"),
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
            3951001893725728678,
            17561894908598795415,
            3273299927427316065,
            4016781436536637665,
            3759867147464905433,
            4273494230197193221,
            3529531907751757055,
            16273736933959562170,
            16977210453145070413,
            4260534243702315869,
            8876721923944456293,
            6706553457839696430,
            11459371310689979744,
            17420813315993560429,
            16645214173008843092,
            1335969637496639684,
        ]);
        // NOTE: generated on Rust Playground
        let test2 = BlockP::new_with2([
            9449436712766709104,
            3648953883981184573,
            14898637992720905965,
            17363463440617121051,
            7750060861933093186,
            14007631929040371275,
            5938564052276943847,
            10629746254474597517,
            3232167171266494280,
            4891434532817971135,
            14814410512354217645,
            16902468201008627571,
            15996213338535303994,
            2018280331266639914,
            3514537016880298159,
            17460098548274586993,
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
