use super::block::BlockP;
use super::wire::WireLabelInternal;
use super::wire_labels_set_bitslice::WireLabelsSetBitSlice;
use super::wire_labels_set_bitslice::WireLabelsSetBitsSliceInternal;
use super::GarblerError;

///
/// About: `clippy::large_enum_variant`: Most gates in a circuit should be binary ones, so using a Box
/// is probably counter productive <https://rust-lang.github.io/rust-clippy/master/index.html#/large_enum_variant>
#[derive(Debug, PartialEq, Clone)]
#[allow(clippy::large_enum_variant)]
pub(super) enum WireLabelsSetInternal {
    BinaryGate {
        x00: WireLabelInternal,
        x01: WireLabelInternal,
        x10: WireLabelInternal,
        x11: WireLabelInternal,
    },
    UnaryGate {
        x0: WireLabelInternal,
        x1: WireLabelInternal,
    },
}

/// "a set of input wire labels X"
/// For a givel Wire with (L0, L1) this will represent the combination
/// X00 = (L0, L0)
/// X01 = (L0, L1)
/// X10 = (L1, L0)
/// X11 = (L1, L1)
///
/// Also noted in <https://www.esat.kuleuven.be/cosic/publications/article-3351.pdf>
/// "The Label Sampling Function f0 This function assigns an l-bit label Kj to
/// each possible value that wire j can take. Collectively, the set of labels associated
/// with the wire is denoted by {Kj }. In particular, Yao’s scheme and all subsequent
/// optimizations decompose the circuit’s input into bits and each bit is assigned a
/// label (See also [App17]).""
///
pub(super) struct WireLabelsSet {
    pub(crate) internal: WireLabelsSetInternal,
}

impl WireLabelsSet {
    pub(crate) fn new_binary(x00: BlockP, x01: BlockP, x10: BlockP, x11: BlockP) -> Self {
        assert_four_different(&x00, &x01, &x10, &x11);
        Self {
            internal: WireLabelsSetInternal::BinaryGate {
                x00: WireLabelInternal { label: x00 },
                x01: WireLabelInternal { label: x01 },
                x10: WireLabelInternal { label: x10 },
                x11: WireLabelInternal { label: x11 },
            },
        }
    }

    pub(crate) fn new_unary(x0: BlockP, x1: BlockP) -> Self {
        assert_ne!(&x0, &x1, "a and b are equal");
        Self {
            internal: WireLabelsSetInternal::UnaryGate {
                x0: WireLabelInternal { label: x0 },
                x1: WireLabelInternal { label: x1 },
            },
        }
    }

    /// In <https://eprint.iacr.org/2021/739.pdf> this is a helper for
    /// "Algorithm 5 Gate"
    /// 7: Set slice ← Xg00[j]||Xg01[j]||Xg10[j]||Xg11[j]
    ///
    /// Return the specific BIT for each x00,x01,x10,x11
    pub(super) fn get_bits_slice(
        &self,
        index: usize,
    ) -> Result<WireLabelsSetBitSlice, GarblerError> {
        match &self.internal {
            WireLabelsSetInternal::BinaryGate { x00, x01, x10, x11 } => Ok(WireLabelsSetBitSlice {
                internal: WireLabelsSetBitsSliceInternal::BinaryGate {
                    x00: x00.get_block().get_bit(index)?,
                    x01: x01.get_block().get_bit(index)?,
                    x10: x10.get_block().get_bit(index)?,
                    x11: x11.get_block().get_bit(index)?,
                },
            }),
            WireLabelsSetInternal::UnaryGate { x0, x1 } => Ok(WireLabelsSetBitSlice {
                internal: WireLabelsSetBitsSliceInternal::UnaryGate {
                    x0: x0.get_block().get_bit(index)?,
                    x1: x1.get_block().get_bit(index)?,
                },
            }),
        }
    }

    pub(super) fn get_x00(&self) -> &BlockP {
        match &self.internal {
            WireLabelsSetInternal::BinaryGate {
                x00,
                x01: _,
                x10: _,
                x11: _,
            } => x00.get_block(),
            WireLabelsSetInternal::UnaryGate { x0: _, x1: _ } => {
                unimplemented!("CompressedSetInternal::UnaryGate")
            }
        }
    }

    pub(super) fn get_x01(&self) -> &BlockP {
        match &self.internal {
            WireLabelsSetInternal::BinaryGate {
                x00: _,
                x01,
                x10: _,
                x11: _,
            } => x01.get_block(),
            WireLabelsSetInternal::UnaryGate { x0: _, x1: _ } => {
                unimplemented!("CompressedSetInternal::UnaryGate")
            }
        }
    }

    #[allow(dead_code)]
    pub(super) fn get_x10(&self) -> &BlockP {
        match &self.internal {
            WireLabelsSetInternal::BinaryGate {
                x00: _,
                x01: _,
                x10,
                x11: _,
            } => x10.get_block(),
            WireLabelsSetInternal::UnaryGate { x0: _, x1: _ } => {
                unimplemented!("CompressedSetInternal::UnaryGate")
            }
        }
    }

    pub(super) fn get_x11(&self) -> &BlockP {
        match &self.internal {
            WireLabelsSetInternal::BinaryGate {
                x00: _,
                x01: _,
                x10: _,
                x11,
            } => x11.get_block(),
            WireLabelsSetInternal::UnaryGate { x0: _, x1: _ } => {
                unimplemented!("CompressedSetInternal::UnaryGate")
            }
        }
    }

    pub(super) fn get_x0(&self) -> &BlockP {
        match &self.internal {
            WireLabelsSetInternal::BinaryGate {
                x00: _,
                x01: _,
                x10: _,
                x11: _,
            } => {
                unimplemented!("CompressedSetInternal::BinaryGate")
            }
            WireLabelsSetInternal::UnaryGate { x0, x1: _ } => x0.get_block(),
        }
    }

    pub(super) fn get_x1(&self) -> &BlockP {
        match &self.internal {
            WireLabelsSetInternal::BinaryGate {
                x00: _,
                x01: _,
                x10: _,
                x11: _,
            } => {
                unimplemented!("CompressedSetInternal::BinaryGate")
            }
            WireLabelsSetInternal::UnaryGate { x0: _, x1 } => x1.get_block(),
        }
    }
}

fn assert_four_different(a: &BlockP, b: &BlockP, c: &BlockP, d: &BlockP) {
    assert_ne!(a, b, "a and b are equal");
    assert_ne!(a, c, "a and c are equal");
    assert_ne!(a, d, "a and d are equal");
    assert_ne!(b, c, "b and c are equal");
    assert_ne!(b, d, "b and d are equal");
    assert_ne!(c, d, "c and d are equal");
}

#[cfg(test)]
mod tests {

    use super::*;

    /// cf tests in lib-garble-rs/src/new_garbling_scheme/block.rs
    /// for a way to generate new BlockP in case of refactor
    ///
    fn get_test_blocks() -> (BlockP, BlockP, BlockP, BlockP) {
        // NOTE: generated on Rust Playground
        let test1 = BlockP::new_with2([
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
        let test2 = BlockP::new_with2([
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
        let test3 = BlockP::new_with2([
            3_273_299_927_427_316_065,
            3_648_953_883_981_184_573,
            14_898_637_992_720_905_965,
            17_363_463_440_617_121_051,
            7_750_060_861_933_093_186,
            14_007_631_929_040_371_275,
            5_938_564_052_276_943_847,
            10_629_746_254_474_597_517,
            3_232_167_171_266_494_280,
            4_891_434_532_817_971_135,
            16_902_468_201_008_627_571,
            16_902_468_201_008_627_571,
            15_996_213_338_535_303_994,
            2_018_280_331_266_639_914,
            3_514_537_016_880_298_159,
            9_449_436_712_766_709_104,
        ]);
        let test4 = BlockP::new_with2([
            3_273_299_927_427_316_065,
            3_648_953_883_981_184_573,
            14_898_637_992_720_905_965,
            17_363_463_440_617_121_051,
            3_273_299_927_427_316_065,
            3_648_953_883_981_184_573,
            14_898_637_992_720_905_965,
            17_363_463_440_617_121_051,
            3_273_299_927_427_316_065,
            3_648_953_883_981_184_573,
            14_898_637_992_720_905_965,
            17_363_463_440_617_121_051,
            3_273_299_927_427_316_065,
            3_648_953_883_981_184_573,
            14_898_637_992_720_905_965,
            17_363_463_440_617_121_051,
        ]);

        (test1, test2, test3, test4)
    }

    fn get_new_binary() -> (WireLabelsSet, BlockP, BlockP, BlockP, BlockP) {
        let (test1, test2, test3, test4) = get_test_blocks();
        let wire_labels_set =
            WireLabelsSet::new_binary(test1.clone(), test2.clone(), test3.clone(), test4.clone());

        (wire_labels_set, test1, test2, test3, test4)
    }

    fn get_new_unary() -> (WireLabelsSet, BlockP, BlockP) {
        let (test1, test2, _test3, _test4) = get_test_blocks();
        let wire_labels_set = WireLabelsSet::new_unary(test1.clone(), test2.clone());

        (wire_labels_set, test1, test2)
    }

    #[test]
    fn test_get_x00() {
        let (wire_labels_set, test1, _test2, _test3, _test4) = get_new_binary();

        assert_eq!(wire_labels_set.get_x00().clone(), test1);
    }

    #[test]
    fn test_get_x01() {
        let (wire_labels_set, _test1, test2, _test3, _test4) = get_new_binary();

        assert_eq!(wire_labels_set.get_x01().clone(), test2);
    }

    #[test]
    fn test_get_x10() {
        let (wire_labels_set, _test1, _test2, test3, _test4) = get_new_binary();

        assert_eq!(wire_labels_set.get_x10().clone(), test3);
    }

    #[test]
    fn test_get_x11() {
        let (wire_labels_set, _test1, _test2, _test3, test4) = get_new_binary();

        assert_eq!(wire_labels_set.get_x11().clone(), test4);
    }

    #[test]
    fn test_get_x0() {
        let (wire_labels_set, test1, _test2) = get_new_unary();

        assert_eq!(wire_labels_set.get_x0().clone(), test1);
    }

    #[test]
    fn test_get_x1() {
        let (wire_labels_set, _test1, test2) = get_new_unary();

        assert_eq!(wire_labels_set.get_x1().clone(), test2);
    }
}
