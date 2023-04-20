use num_enum::TryFromPrimitive;

/// This is a "reference" to either:
/// - another Gate's inputs
/// - a Gate's output
/// - a Circuit's output
#[derive(Clone)]
pub(crate) struct GateRef {
    pub(crate) id: usize,
}

/// All the Gates type possible in SKCD file format
///
/// SHOULD match
/// - `enum SkcdGateType` from skcd.proto
/// - `lib_circuits/src/blif/gate_types.h`
/// - `lib_garble/src/justgarble/gate_types.h`
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, TryFromPrimitive)]
#[repr(i32)]
pub(crate) enum GateType {
    ZERO = 0,
    NOR = 1,
    /// A-and-not-B
    AANB = 2,
    /// NOT B
    INVB = 3,
    /// not-A-and-B?
    NAAB = 4,
    /// NOT A
    INV = 5,
    XOR = 6,
    NAND = 7,
    AND = 8,
    XNOR = 9,
    BUF = 10,
    /// A-or-NOT-B?
    AONB = 11,
    BUFB = 12,
    /// NOT-A-or-B?
    NAOB = 13,
    OR = 14,
    ONE = 15,
}

// TODO use ?
// enum SkcdInput {
//     Garbler,
//     Evaluator,
//     /// Default: means the input is another gate's output
//     Default,
// }

/// For now in .skcd we have two kind of gates:
/// - standard eg: "8 = XOR(7,2)        // 8 = 7 xor Cin"
/// - constant eg: "3 = ZERO(0,0)" or "5 = ONE(0,0)"
/// Which means Constant type only has an output and NO input.
///
/// NOTE: it SHOULD be optimized-out by Verilog/ABC but right now, we CAN have multiple ZERO and ONE gates in a Circuit!
pub(crate) enum GateInternal {
    Standard {
        r#type: GateType,
        input_a: Option<GateRef>,
        input_b: Option<GateRef>,
    },
    Constant {
        value: bool,
    },
}

pub(crate) struct Gate {
    pub(crate) internal: GateInternal,
    /// Gate's output is in practice a Gate's ID or idx
    pub(crate) output: GateRef,
}
