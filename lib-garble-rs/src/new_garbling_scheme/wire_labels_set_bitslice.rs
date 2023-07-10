use super::wire_value::WireValue;

#[derive(Debug, PartialEq, Clone)]
pub(super) enum WireLabelsSetBitsSliceInternal {
    BinaryGate {
        x00: WireValue,
        x01: WireValue,
        x10: WireValue,
        x11: WireValue,
    },
    UnaryGate {
        x0: WireValue,
        x1: WireValue,
    },
}

/// Represent a "bit slice" for a given `WireLabelsSet`
/// eg for a given `WireLabelsSet`
/// X00: [1, 0, 0, 1, ...]
/// X01: [1, 0, 0, 1, ...]
/// [...]
///
/// This will for example represent [1, 1] if we slice at the first bit
/// or [0, 0] if we slice at the second bit, etc
///
#[derive(Debug, PartialEq, Clone)]
pub(super) struct WireLabelsSetBitSlice {
    pub(super) internal: WireLabelsSetBitsSliceInternal,
}

impl WireLabelsSetBitSlice {
    #[allow(clippy::fn_params_excessive_bools)]
    pub(super) fn new_binary_gate_from_bool(x00: bool, x01: bool, x10: bool, x11: bool) -> Self {
        Self {
            internal: WireLabelsSetBitsSliceInternal::BinaryGate {
                x00: x00.into(),
                x01: x01.into(),
                x10: x10.into(),
                x11: x11.into(),
            },
        }
    }

    pub(super) fn new_unary_gate_from_bool(x0: bool, x1: bool) -> Self {
        Self {
            internal: WireLabelsSetBitsSliceInternal::UnaryGate {
                x0: x0.into(),
                x1: x1.into(),
            },
        }
    }
}
