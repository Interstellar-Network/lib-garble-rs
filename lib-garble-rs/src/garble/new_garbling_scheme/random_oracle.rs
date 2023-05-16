use bitvec::prelude::*;
use rand::seq::SliceRandom;
use rand::Rng;
use rand_chacha::{rand_core::SeedableRng, ChaChaRng};

use super::block::{BlockL, BlockP, KAPPA_BYTES};
use super::constant::{KAPPA, KAPPA_FACTOR};

use super::WireValue;

pub(crate) struct RandomOracle {
    rng: ChaChaRng,
}

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
    // TODO should probably be deterministic? or random?
    // use some kind of hash?
    // TODO! should this instead a `l_prime` length Block (== 8*KAPPA)???
    pub(super) fn random_oracle_g(label_a: &BlockL, label_b: &BlockL, tweak: usize) -> BlockP {
        // TODO! which hash to use? sha2, sha256?
        // or maybe some MAC? cf `keyed_hash`?
        // TODO! how to properly pass "tweak"?
        let tweak_bytes = tweak.to_le_bytes();
        let mut hasher = blake3::Hasher::new();
        hasher.update(&tweak_bytes);
        hasher.update(label_a.as_bytes());
        hasher.update(label_b.as_bytes());
        // TODO! what do we do with a 256bits hash but a 128bits Block?
        let mut hash2 = hasher.finalize_xof();
        // TODO! is filling 8 * 128 bits OK from a 256 bits hash???
        let mut hash2_bytes = [0u8; KAPPA_BYTES * KAPPA_FACTOR];
        hash2.fill(&mut hash2_bytes);

        BlockP::new_with2(hash2_bytes)
    }

    pub(super) fn new_random_blockL(&mut self) -> BlockL {
        let arr1: [u64; 2] = self.rng.gen();
        BlockL::new_with(arr1)
    }

    pub(super) fn new() -> Self {
        Self {
            rng: ChaChaRng::from_entropy(),
        }
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
    pub(super) fn random_oracle_prime(&self, l0_l1: &BlockL, dj: &BlockL) -> bool {
        // TODO(random_oracle) what should we use here???
        // l0_l1.lsb(dj)

        let mut hasher = blake3::Hasher::new();
        hasher.update(l0_l1.as_bytes());
        hasher.update(dj.as_bytes());
        // TODO! what do we do with a 256bits hash but a 128bits Block?
        let mut hash2 = hasher.finalize();

        // Extract the least significant bit from the hash
        let last_byte = hash2.as_bytes()[hash2.as_bytes().len() - 1];
        // (last_byte & 1) => is a u8
        // so Convert u8 -> bool
        (last_byte & 1) == 1
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

    #[test]
    fn test_random_oracle_0_same_blocks_different_tweaks_should_return_different_hashes() {
        let block_a = BlockL::new_with([42, 0]);
        let block_b = BlockL::new_with([43, 44]);

        let hash1 = RandomOracle::random_oracle_g(&block_a, &block_b, 0);
        let hash2 = RandomOracle::random_oracle_g(&block_a, &block_b, 1);

        assert_ne!(hash1, hash2, "returning hashes SHOULD NOT be equal!");
    }

    #[test]
    fn test_random_oracle_0_same_blocks_same_tweaks_should_return_same_hashes() {
        let block_a = BlockL::new_with([42, 0]);
        let block_b = BlockL::new_with([43, 44]);

        let hash1 = RandomOracle::random_oracle_g(&block_a, &block_b, 2);
        let hash2 = RandomOracle::random_oracle_g(&block_a, &block_b, 2);

        assert_eq!(hash1, hash2, "returning hashes SHOULD be equal!");
    }

    #[test]
    fn test_random_oracle_0_different_blocks_same_tweaks_should_return_different_hashes() {
        let block_a = BlockL::new_with([42, 0]);
        let block_b = BlockL::new_with([43, 44]);

        let hash1 = RandomOracle::random_oracle_g(&block_a, &block_b, 2);
        let hash2 = RandomOracle::random_oracle_g(&block_b, &block_a, 2);

        assert!(hash1 != hash2, "returning hashes SHOULD NOT be equal!");
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
