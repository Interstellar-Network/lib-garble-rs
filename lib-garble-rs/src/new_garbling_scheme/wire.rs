use serde::{Deserialize, Serialize};

use super::block::{BlockL, BlockP};

/// Represent either the TRUE or the FALSE part of a `Wire`
///
/// This is also used during evaluation
/// the `value` SHOULD match either a `Wire.value0` OR a `Wire.value1`
///
// TODO do this ^^^^ -> `value` SHOULD be ref
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WireLabel {
    label: BlockL,
}

impl WireLabel {
    pub(super) fn new(block: &BlockL) -> Self {
        Self {
            label: block.clone(),
        }
    }

    pub(super) fn get_block(&self) -> &BlockL {
        &self.label
    }
}

/// Like `WireLabel` by INTERNAL part
/// So based on `l'` length block instead of `l`
#[derive(Debug, Clone, PartialEq)]
pub(super) struct WireLabelInternal {
    pub(super) label: BlockP,
}

impl WireLabelInternal {
    pub(super) fn get_block(&self) -> &BlockP {
        &self.label
    }
}

/// Called "wire label set W" in https://eprint.iacr.org/2021/739.pdf
/// This is a pair of random label of l-size, one representing a 0 on the Wire,
/// and one for 1.
///
/// Alternatively noted "Collectively, the set of labels associated with the wire is denoted by {Kj}"
/// in https://www.esat.kuleuven.be/cosic/publications/article-3351.pdf
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct Wire {
    label0: WireLabel,
    label1: WireLabel,
}

impl Wire {
    /// Create a new `Wire`
    ///
    /// `value0` and `value1` MUST be different!
    pub(super) fn new(label0: BlockL, label1: BlockL) -> Self {
        // FAIL technically here we don't care if they are the same
        // BUT in `decoding_info` we loop until both the LSB of left and not right are different
        // and it they are the same here -> infinite loop!
        assert!(label0 != label1, "`value0` and `value1` MUST be different!");
        Self {
            label0: WireLabel { label: label0 },
            label1: WireLabel { label: label1 },
        }
    }

    pub(super) fn value0(&self) -> &BlockL {
        &self.label0.get_block()
    }

    pub(super) fn value1(&self) -> &BlockL {
        &self.label1.get_block()
    }
}
