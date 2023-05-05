use rand::seq::SliceRandom;
use rand::Rng;
use rand_chacha::{rand_core::SeedableRng, ChaChaRng};

use crate::garble::new_garbling_scheme::constant::KAPPA;
use crate::garble::new_garbling_scheme::Block;

pub(crate) struct RandomOracle {
    rng: ChaChaRng,
}

impl RandomOracle {
    // TODO should probably be deterministic? or random?
    // use some kind of hash?
    pub(crate) fn random_oracle(&self, label_a: &Block, label_b: &Block) -> Block {
        // TODO! which hash to use? sha2, sha256?
        // or maybe some MAC? cf `keyed_hash`?
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"bar");
        hasher.update(b"baz");
        let hash2 = hasher.finalize();
        // TODO! what do we do with a 256bits hash but a 128bits Block?
        let hash2_bytes: [u8; 16] = hash2.as_bytes()[0..16].try_into().unwrap();

        Block::new_with2(hash2_bytes)
    }

    pub(crate) fn new_random_block(&mut self) -> Block {
        let arr1: [u64; 2] = self.rng.gen();
        Block::new_with(arr1)
    }

    pub(crate) fn new() -> Self {
        Self {
            rng: ChaChaRng::from_entropy(),
        }
    }
}
