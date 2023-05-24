use super::block::BlockL;

/// Represent either the TRUE or the FALSE part of a `Wire`
///
/// This is also used during evaluation
/// the `value` SHOULD match either a `Wire.value0` OR a `Wire.value1`
///
// TODO do this ^^^^ -> `value` SHOULD be ref
#[derive(Debug, Clone)]
pub(super) struct WireLabel {
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

/// Called "wire label set W" in https://eprint.iacr.org/2021/739.pdf
/// This is a pair of random label of l-size, one representing a 0 on the Wire,
/// and one for 1.
///
/// Alternatively noted "Collectively, the set of labels associated with the wire is denoted by {Kj}"
/// in https://www.esat.kuleuven.be/cosic/publications/article-3351.pdf
#[derive(Debug, Clone)]
pub(super) struct Wire {
    label0: WireLabel,
    label1: WireLabel,
}

impl Wire {
    /// Create a new `Wire`
    ///
    /// `value0` and `value1` MUST be different!
    pub(super) fn new(label0: BlockL, label1: BlockL) -> Self {
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
