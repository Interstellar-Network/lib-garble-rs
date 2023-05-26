use super::block::BlockP;
use super::CompressedSetBitSlice;
use super::CompressedSetBitSliceInternal;

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum WireLabelsSetInternal {
    BinaryGate {
        x00: BlockP,
        x01: BlockP,
        x10: BlockP,
        x11: BlockP,
    },
    UnaryGate {
        x0: BlockP,
        x1: BlockP,
    },
}

/// "a set of input wire labels X"
/// For a givel Wire with (L0, L1) this will represent the combination
/// X00 = (L0, L0)
/// X01 = (L0, L1)
/// X10 = (L1, L0)
/// X11 = (L1, L1)
///
/// Also noted in https://www.esat.kuleuven.be/cosic/publications/article-3351.pdf
/// "The Label Sampling Function f0 This function assigns an l-bit label Kj to
/// each possible value that wire j can take. Collectively, the set of labels associated
/// with the wire is denoted by {Kj }. In particular, Yao’s scheme and all subsequent
/// optimizations decompose the circuit’s input into bits and each bit is assigned a
/// label (See also [App17]).""
///
pub(crate) struct WireLabelsSet {
    pub(crate) internal: WireLabelsSetInternal,
}

pub(crate) fn assert_four_different(a: &BlockP, b: &BlockP, c: &BlockP, d: &BlockP) {
    assert_ne!(a, b, "a and b are equal");
    assert_ne!(a, c, "a and c are equal");
    assert_ne!(a, d, "a and d are equal");
    assert_ne!(b, c, "b and c are equal");
    assert_ne!(b, d, "b and d are equal");
    assert_ne!(c, d, "c and d are equal");
}

impl WireLabelsSet {
    pub(crate) fn new_binary(x00: BlockP, x01: BlockP, x10: BlockP, x11: BlockP) -> Self {
        assert_four_different(&x00, &x01, &x10, &x11);
        Self {
            internal: WireLabelsSetInternal::BinaryGate { x00, x01, x10, x11 },
        }
    }

    pub(crate) fn new_unary(x0: BlockP, x1: BlockP) -> Self {
        assert_ne!(&x0, &x1, "a and b are equal");
        Self {
            internal: WireLabelsSetInternal::UnaryGate { x0, x1 },
        }
    }

    /// In https://eprint.iacr.org/2021/739.pdf this is a helper for
    /// "Algorithm 5 Gate"
    /// 7: Set slice ← Xg00[j]||Xg01[j]||Xg10[j]||Xg11[j]
    ///
    /// Return the specific BIT for each x00,x01,x10,x11
    pub(super) fn get_bits_slice(&self, index: usize) -> CompressedSetBitSlice {
        match &self.internal {
            WireLabelsSetInternal::BinaryGate { x00, x01, x10, x11 } => CompressedSetBitSlice {
                internal: CompressedSetBitSliceInternal::BinaryGate {
                    x00: x00.get_bit(index),
                    x01: x01.get_bit(index),
                    x10: x10.get_bit(index),
                    x11: x11.get_bit(index),
                },
            },
            WireLabelsSetInternal::UnaryGate { x0, x1 } => CompressedSetBitSlice {
                internal: CompressedSetBitSliceInternal::UnaryGate {
                    x0: x0.get_bit(index),
                    x1: x1.get_bit(index),
                },
            },
        }
    }

    pub(super) fn get_x00(&self) -> &BlockP {
        match &self.internal {
            WireLabelsSetInternal::BinaryGate { x00, x01, x10, x11 } => x00,
            WireLabelsSetInternal::UnaryGate { x0, x1 } => {
                unimplemented!("CompressedSetInternal::UnaryGate")
            }
        }
    }

    pub(super) fn get_x01(&self) -> &BlockP {
        match &self.internal {
            WireLabelsSetInternal::BinaryGate { x00, x01, x10, x11 } => x01,
            WireLabelsSetInternal::UnaryGate { x0, x1 } => {
                unimplemented!("CompressedSetInternal::UnaryGate")
            }
        }
    }

    pub(super) fn get_x10(&self) -> &BlockP {
        match &self.internal {
            WireLabelsSetInternal::BinaryGate { x00, x01, x10, x11 } => x10,
            WireLabelsSetInternal::UnaryGate { x0, x1 } => {
                unimplemented!("CompressedSetInternal::UnaryGate")
            }
        }
    }

    pub(super) fn get_x11(&self) -> &BlockP {
        match &self.internal {
            WireLabelsSetInternal::BinaryGate { x00, x01, x10, x11 } => x11,
            WireLabelsSetInternal::UnaryGate { x0, x1 } => {
                unimplemented!("CompressedSetInternal::UnaryGate")
            }
        }
    }

    pub(super) fn get_x0(&self) -> &BlockP {
        match &self.internal {
            WireLabelsSetInternal::BinaryGate { x00, x01, x10, x11 } => {
                unimplemented!("CompressedSetInternal::BinaryGate")
            }
            WireLabelsSetInternal::UnaryGate { x0, x1 } => x0,
        }
    }

    pub(super) fn get_x1(&self) -> &BlockP {
        match &self.internal {
            WireLabelsSetInternal::BinaryGate { x00, x01, x10, x11 } => {
                unimplemented!("CompressedSetInternal::BinaryGate")
            }
            WireLabelsSetInternal::UnaryGate { x0, x1 } => x1,
        }
    }
}
