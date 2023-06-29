use core::mem::size_of;

use bitvec::prelude::*;
use bytes::BytesMut;
use rand::Rng;
use rand_chacha::{rand_core::SeedableRng, ChaChaRng};
use xxhash_rust::xxh3::xxh3_128;

use super::block::{BitsInternal, BlockL, BlockP, MyBitArrayL, KAPPA_NB_ELEMENTS};
use super::constant::KAPPA_FACTOR;

pub(crate) struct RandomOracle {}

impl RandomOracle {
    /// First Random Oracle = RO0
    /// ROg : {0, 1}nℓ → {0, 1}ℓ′ in https://eprint.iacr.org/2021/739.pdf
    /// "The random oracle
    /// RO takes as input the tweak g and two labels with total length 2ℓ, and outputs
    /// an ℓ′-length string"
    /// "The random oracle RO employed throughout the gate-by-gate
    /// garbling process is tweakable: it takes as an additional input the gate index g so
    /// that it behaves independently for each gate."
    ///
    /// param:
    /// - `label_b` is optional; that way we use this RO for both binary and unary Gates
    ///
    // TODO should probably be deterministic? or random?
    // use some kind of hash?
    // TODO! should this instead a `l_prime` length Block (== 8*KAPPA)???
    pub(super) fn random_oracle_g(
        label_a: &BlockL,
        label_b: Option<&BlockL>,
        tweak: usize,
    ) -> BlockP {
        let hash_0 = Self::random_oracle_g_core(label_a, label_b, tweak);

        // We need to construct the final `[u8; 128]` so for now we just concat
        // `[u8; 128]` == `[0u8; KAPPA_NB_ELEMENTS * KAPPA_FACTOR * size_of::<BitsInternal>()]`
        // -> We should re-hash in loop: https://github.com/Cyan4973/xxHash/issues/680
        //
        // TODO! is filling 8 * 128 bits OK from a 128 bits hash???
        let hash_1 = xxh3_128(&hash_0.to_be_bytes());
        let hash_2 = hash_1 ^ hash_0;
        let hash_3 = hash_2 ^ hash_0;
        let hash_4 = hash_3 ^ hash_0;
        let hash_5 = hash_4 ^ hash_0;
        let hash_6 = hash_5 ^ hash_0;
        let hash_7 = hash_6 ^ hash_0;

        let mut hash_bytes_big: [u8; 128] = [
            hash_0.to_le_bytes(),
            hash_1.to_le_bytes(),
            hash_2.to_le_bytes(),
            hash_3.to_le_bytes(),
            hash_4.to_le_bytes(),
            hash_5.to_le_bytes(),
            hash_6.to_le_bytes(),
            hash_7.to_le_bytes(),
        ]
        .concat()
        .try_into()
        .unwrap();

        BlockP::new_with_raw_bytes(hash_bytes_big)
    }

    /// "Truncated" version of `random_oracle_g`
    /// This is used by eval to avoid allocating a BlockP just to convert(ie truncate) it
    /// into a BlockL right after.
    /// Doing it that way avoids both an alloc, and more important: 7 rounds of xxh3_128(or XOR)
    pub(super) fn random_oracle_g_truncated(
        label_a: &BlockL,
        label_b: Option<&BlockL>,
        tweak: usize,
    ) -> BlockL {
        let hash_0 = Self::random_oracle_g_core(label_a, label_b, tweak);

        // https://stackoverflow.com/questions/75746412/copy-a-u128-into-u642
        let words: MyBitArrayL = unsafe { std::mem::transmute::<u128, MyBitArrayL>(hash_0) };

        BlockL::new_with(words)
    }

    fn random_oracle_g_core(label_a: &BlockL, label_b: Option<&BlockL>, tweak: usize) -> u128 {
        // TODO! which hash to use? sha2, sha256?
        // or maybe some MAC? cf `keyed_hash`?
        // TODO! how to properly pass "tweak"?
        let tweak_bytes = tweak.to_le_bytes();
        let data = if let Some(label_b_block) = label_b {
            [
                tweak_bytes.as_slice(),
                label_a.as_bytes(),
                label_b_block.as_bytes(),
            ]
            .concat()
        } else {
            [tweak_bytes.as_slice(), label_a.as_bytes()].concat()
        };

        xxh3_128(&data)
    }

    pub(super) fn new_random_block_l(rng: &mut ChaChaRng) -> BlockL {
        let arr1: [BitsInternal; KAPPA_NB_ELEMENTS] = rng.gen();
        BlockL::new_with(arr1)
    }

    ///
    /// In: https://eprint.iacr.org/2021/739.pdf
    /// "In our construction, we employ another
    /// random oracle RO′ for this. In the subroutine that creates the decoding informa-
    /// tion, for every output wire j, we sample an ℓ-bit string dj . This string has the
    /// property that, given output wire labels (Lj0, Lj1), it holds that RO′(Lj0, dj ) = 0
    /// and RO′(Lj1, dj ) = 1. Note that such a decoding will always yield some out-
    /// put even for arbitrary ℓ-bit strings that are not output labels.
    /// The subroutine DecodingInfo(D) → d generates this decoding information given the output wirelabels set."
    ///
    /// (2) RO′ : {0, 1}2ℓ → {0, 1}
    /// See also: "Algorithm 6 DecodingInfo(D, ℓ)"
    ///
    /// param:
    /// - `L0` or `L1` Block for the current output Gate
    pub(super) fn random_oracle_prime(l0_l1: &BlockL, dj: &BlockL, buf: &mut BytesMut) -> bool {
        // prepare the data: append `l0_l1` with `dj`
        // reuse `buf` to avoid alloc!
        buf.clear();
        let l0_l1_bytes = l0_l1.as_bytes();
        let dj_bytes = dj.as_bytes();
        buf.reserve(l0_l1_bytes.len() + dj_bytes.len());
        buf.extend_from_slice(l0_l1.as_bytes());
        buf.extend_from_slice(dj.as_bytes());
        let hash = xxh3_128(&buf);

        // Extract the least significant bit from the hash
        // Technically we DO NOT need the LSB; we just need to be consistant b/w garbling and eval
        // ie we DO NOT care about big endian vs little endian
        let x = hash & 1;
        x == 1

        // // Extract the least significant bit from the hash
        // // let last_byte = hash2.as_bytes()[hash2.as_bytes().len() - 1];
        // // FAIL: the internal buffer is 64 bytes, but at this point only 16+16 are filled
        // // so it always extracts a 0? --> NO! random-ish byte, but clearly when masking with `& 1` after
        // // this is NOT random at all; mostly a true as a result!
        // let hash_bytes = hash.to_le_bytes();

        // // TODO????
        // // let last_byte = hash_bytes[hash_bytes.len() / 2];
        // // (last_byte & 1) => is a u8
        // // so Convert u8 -> bool
        // // (last_byte >> 8) & 1
        // // (1 << 8) & last_byte

        // let bits = hash_bytes.view_bits::<Lsb0>();
        // let x = *bits.last().unwrap();

        // // println!("random_oracle_prime: {:?}", x);
        // x
    }

    // /// Second Random Oracle = RO1
    // /// "However, our second optimization shows that that this is unnecessary. Instead
    // /// of sampling new labels KC0 and KC1, we can derive them directly from the values
    // /// S0 and S1, even if the later have fewer than ` bits of entropy (as long as they
    // /// have κ bits of entropy)."
    // ///
    // /// Used to generate:
    // /// KC0 = RO1(S0)
    // /// KC1 = RO1(S1)
    // pub(super) fn random_oracle_1(sblock: &[WireValue]) -> BlockP {
    //     // convert the &[bool] -> &[u8]
    //     let mut bv = bitvec![u8, Msb0;];
    //     for bit in sblock.into_iter() {
    //         bv.push(*bit);
    //     }

    //     // TODO! which hash to use? sha2, sha256?
    //     // or maybe some MAC? cf `keyed_hash`?
    //     let mut hasher = blake3::Hasher::new();
    //     hasher.update(bv.as_raw_slice());
    //     let mut hash2 = hasher.finalize_xof();
    //     // TODO! is filling 8 * 128 bits OK from a 256 bits hash???
    //     let mut hash2_bytes = [0u8; KAPPA_BYTES * KAPPA_FACTOR];
    //     hash2.fill(&mut hash2_bytes);

    //     BlockP::new_with2(hash2_bytes)
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_blocks() -> (BlockL, BlockL, BlockL) {
        let block_a = BlockL::new_with([42, 0]);
        let block_b = BlockL::new_with([43, 44]);
        let block_common = BlockL::new_with([11, 12]);

        (block_a, block_b, block_common)
    }

    #[test]
    fn test_random_oracle_0_same_blocks_different_tweaks_should_return_different_hashes() {
        let (block_a, block_b, block_common) = get_test_blocks();

        let hash1 = RandomOracle::random_oracle_g(&block_a, Some(&block_b), 0);
        let hash2 = RandomOracle::random_oracle_g(&block_a, Some(&block_b), 1);

        assert_ne!(hash1, hash2, "returning hashes SHOULD NOT be equal!");
    }

    #[test]
    fn test_random_oracle_0_same_blocks_same_tweaks_should_return_same_hashes() {
        let (block_a, block_b, block_common) = get_test_blocks();

        let hash1 = RandomOracle::random_oracle_g(&block_a, Some(&block_b), 2);
        let hash2 = RandomOracle::random_oracle_g(&block_a, Some(&block_b), 2);

        assert_eq!(hash1, hash2, "returning hashes SHOULD be equal!");
    }

    #[test]
    fn test_random_oracle_0_different_blocks_same_tweaks_should_return_different_hashes() {
        let (block_a, block_b, block_common) = get_test_blocks();

        let hash1 = RandomOracle::random_oracle_g(&block_a, Some(&block_b), 2);
        let hash2 = RandomOracle::random_oracle_g(&block_b, Some(&block_a), 2);

        assert!(hash1 != hash2, "returning hashes SHOULD NOT be equal!");
    }

    #[test]
    fn test_random_oracle_0_different_blocks_same_tweaks_should_return_different_hashes_2() {
        let (block_a, block_b, block_common) = get_test_blocks();

        let hash1 = RandomOracle::random_oracle_g(&block_a, Some(&block_common), 2);
        let hash2 = RandomOracle::random_oracle_g(&block_b, Some(&block_common), 2);

        assert!(hash1 != hash2, "returning hashes SHOULD NOT be equal!");
    }

    #[test]
    fn test_random_oracle_0_different_blocks_same_tweaks_should_return_different_hashes_3() {
        let (block_a, block_b, block_common) = get_test_blocks();

        let hash1 = RandomOracle::random_oracle_g(&block_common, Some(&block_a), 2);
        let hash2 = RandomOracle::random_oracle_g(&block_common, Some(&block_b), 2);

        assert!(hash1 != hash2, "returning hashes SHOULD NOT be equal!");
    }

    #[test]
    fn test_random_oracle_prime_distribution_1() {
        let mut rng = ChaChaRng::from_entropy();

        let mut results = vec![];
        let lj0 = RandomOracle::new_random_block_l(&mut rng);
        let mut buf = BytesMut::new();

        for i in 0..1000 {
            let dj = RandomOracle::new_random_block_l(&mut rng);
            let a = !RandomOracle::random_oracle_prime(&lj0, &dj, &mut buf);
            results.push(a);
        }

        let count_true = results.iter().filter(|&n| *n).count();
        let count_false = results.iter().filter(|&n| !*n).count();
        assert!(count_true.abs_diff(count_false) < 100, "bad distribution!");
    }

    #[test]
    fn test_random_oracle_prime_distribution_2() {
        let mut rng = ChaChaRng::from_entropy();

        let mut results = vec![];
        let dj = RandomOracle::new_random_block_l(&mut rng);
        let mut buf = BytesMut::new();

        for i in 0..1000 {
            let lj0 = RandomOracle::new_random_block_l(&mut rng);
            let a = !RandomOracle::random_oracle_prime(&lj0, &dj, &mut buf);
            results.push(a);
        }

        let count_true = results.iter().filter(|&n| *n).count();
        let count_false = results.iter().filter(|&n| !*n).count();
        assert!(count_true.abs_diff(count_false) < 100, "bad distribution!");
    }

    // #[test]
    // fn test_random_oracle_1_same_blocks_should_return_same_hashes() {
    //     let block_a = vec![true; 16];

    //     let hash1 = RandomOracle::random_oracle_1(&block_a);
    //     let hash2 = RandomOracle::random_oracle_1(&block_a);

    //     assert!(hash1 == hash2, "returning hashes SHOULD be equal!");
    // }

    // #[test]
    // fn test_random_oracle_1_different_blocks_should_return_different_hashes() {
    //     let block_a = vec![true; 16];
    //     let block_b = vec![false; 16];

    //     let hash1 = RandomOracle::random_oracle_1(&block_a);
    //     let hash2 = RandomOracle::random_oracle_1(&block_b);

    //     assert!(hash1 != hash2, "returning hashes SHOULD NOT be equal!");
    // }
}
