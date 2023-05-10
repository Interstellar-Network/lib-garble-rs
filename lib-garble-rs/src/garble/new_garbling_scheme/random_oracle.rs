use bitvec::prelude::*;
use rand::seq::SliceRandom;
use rand::Rng;
use rand_chacha::{rand_core::SeedableRng, ChaChaRng};

use crate::garble::new_garbling_scheme::constant::KAPPA;
use crate::garble::new_garbling_scheme::Block;

use super::WireInternal;

pub(crate) struct RandomOracle {
    rng: ChaChaRng,
}

impl RandomOracle {
    /// First Random Oracle = RO0
    // TODO should probably be deterministic? or random?
    // use some kind of hash?
    // TODO! should this instead a `l_prime` length Block (== 8*KAPPA)???
    pub(super) fn random_oracle_0(label_a: &Block, label_b: &Block, tweak: usize) -> Block {
        // TODO! which hash to use? sha2, sha256?
        // or maybe some MAC? cf `keyed_hash`?
        // TODO! how to properly pass "tweak"?
        let tweak_bytes = tweak.to_le_bytes();
        let mut hasher = blake3::Hasher::new();
        hasher.update(&tweak_bytes);
        hasher.update(label_a.as_bytes());
        hasher.update(label_b.as_bytes());
        let hash2 = hasher.finalize();
        // TODO! what do we do with a 256bits hash but a 128bits Block?
        let hash2_bytes: [u8; 16] = hash2.as_bytes()[0..16].try_into().unwrap();

        Block::new_with2(hash2_bytes)
    }

    pub(super) fn new_random_block(&mut self) -> Block {
        let arr1: [u64; 2] = self.rng.gen();
        Block::new_with(arr1)
    }

    pub(super) fn new() -> Self {
        Self {
            rng: ChaChaRng::from_entropy(),
        }
    }

    /// Second Random Oracle = RO1
    /// "However, our second optimization shows that that this is unnecessary. Instead
    /// of sampling new labels KC0 and KC1, we can derive them directly from the values
    /// S0 and S1, even if the later have fewer than ` bits of entropy (as long as they
    /// have κ bits of entropy)."
    ///
    /// Used to generate:
    /// KC0 = RO1(S0)
    /// KC1 = RO1(S1)
    pub(super) fn random_oracle_1(sblock: &[WireInternal]) -> Block {
        // convert the &[bool] -> &[u8]
        let mut bv = bitvec![u8, Msb0;];
        for bit in sblock.into_iter() {
            bv.push(*bit);
        }

        // TODO! which hash to use? sha2, sha256?
        // or maybe some MAC? cf `keyed_hash`?
        let mut hasher = blake3::Hasher::new();
        hasher.update(bv.as_raw_slice());
        let hash2 = hasher.finalize();
        // TODO! what do we do with a 256bits hash but a 128bits Block?
        let hash2_bytes: [u8; 16] = hash2.as_bytes()[0..16].try_into().unwrap();

        Block::new_with2(hash2_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_oracle_0_same_blocks_different_tweaks_should_return_different_hashes() {
        let block_a = Block::new_with([42, 0]);
        let block_b = Block::new_with([43, 44]);

        let hash1 = RandomOracle::random_oracle_0(&block_a, &block_b, 0);
        let hash2 = RandomOracle::random_oracle_0(&block_a, &block_b, 1);

        assert_ne!(hash1, hash2, "returning hashes SHOULD NOT be equal!");
    }

    #[test]
    fn test_random_oracle_0_same_blocks_same_tweaks_should_return_same_hashes() {
        let block_a = Block::new_with([42, 0]);
        let block_b = Block::new_with([43, 44]);

        let hash1 = RandomOracle::random_oracle_0(&block_a, &block_b, 2);
        let hash2 = RandomOracle::random_oracle_0(&block_a, &block_b, 2);

        assert_eq!(hash1, hash2, "returning hashes SHOULD be equal!");
    }

    #[test]
    fn test_random_oracle_0_different_blocks_same_tweaks_should_return_different_hashes() {
        let block_a = Block::new_with([42, 0]);
        let block_b = Block::new_with([43, 44]);

        let hash1 = RandomOracle::random_oracle_0(&block_a, &block_b, 2);
        let hash2 = RandomOracle::random_oracle_0(&block_b, &block_a, 2);

        assert!(hash1 != hash2, "returning hashes SHOULD NOT be equal!");
    }

    #[test]
    fn test_random_oracle_1_same_blocks_should_return_same_hashes() {
        let block_a = vec![true; 16];

        let hash1 = RandomOracle::random_oracle_1(&block_a);
        let hash2 = RandomOracle::random_oracle_1(&block_a);

        assert!(hash1 == hash2, "returning hashes SHOULD be equal!");
    }

    #[test]
    fn test_random_oracle_1_different_blocks_should_return_different_hashes() {
        let block_a = vec![true; 16];
        let block_b = vec![false; 16];

        let hash1 = RandomOracle::random_oracle_1(&block_a);
        let hash2 = RandomOracle::random_oracle_1(&block_b);

        assert!(hash1 != hash2, "returning hashes SHOULD NOT be equal!");
    }
}
