use num_enum::TryFromPrimitive;

/// This is a "reference" to either:
/// - another Gate's inputs
/// - a Gate's output
/// - a Circuit's output
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub(crate) struct WireRef {
    pub(crate) id: usize,
}

/// All the Gates type possible in SKCD file format
///
/// SHOULD match
/// - `enum SkcdGateType` from skcd.proto
/// - `lib_circuits/src/blif/gate_types.h`
/// - `lib_garble/src/justgarble/gate_types.h`
///
/// IMPORTANT: "ONE" and "ZERO" are special cases: they are mapped to GateInternal::Constant
/// The rest is parsed as-is into a GateInternal::Standard
/*

Can you rewrite all logic gates (eg NAND, NOR, OR, etc) using only XOR and AND (and constant 0 and 1) ?
Answer

It is possible to rewrite all logic gates using only XOR and AND gates, along with constant 0 and 1. Although NAND and NOR gates are commonly referred to as universal gates because any digital circuit can be implemented using just one of these two gates geeksforgeeks.org, we can still derive other gates using XOR and AND gates. Let's take a look at the possible implementations:

    NOT Gate

    A NOT gate can be implemented using XOR gate and a constant 1:

    NOT A = A XOR 1

The truth table for this implementation is:

A | NOT A
---------
0 |   1
1 |   0

OR Gate

An OR gate can be derived using XOR and AND gates (electronics.stackexchange.com):

A OR B = A XOR B XOR (A AND B)

The truth table for this implementation is:

A | B | A OR B
---------------
0 | 0 |   0
0 | 1 |   1
1 | 0 |   1
1 | 1 |   1

NAND Gate

A NAND gate can be implemented using XOR, AND gates, and a constant 1:

A NAND B = (A AND B) XOR 1

The truth table for this implementation is:

A | B | A NAND B
----------------
0 | 0 |   1
0 | 1 |   1
1 | 0 |   1
1 | 1 |   0

NOR Gate

A NOR gate can be implemented using XOR, AND gates, and a constant 1:

A NOR B = (A XOR B) AND (A XOR 1) AND (B XOR 1)

The truth table for this implementation is:

A | B | A NOR B
---------------
0 | 0 |   1
0 | 1 |   0
1 | 0 |   0
1 | 1 |   0

XNOR Gate

An XNOR gate can be implemented using XOR and AND gates:

A XNOR B = (A XOR B) XOR (A AND B)

The truth table for this implementation is:

A | B | A XNOR B
----------------
0 | 0 |   1
0 | 1 |   0
1 | 0 |   0
1 | 1 |   1

In summary, while NAND and NOR gates are commonly used as universal gates, it is possible to derive all logic gates using only XOR and AND gates, along with constant 0 and 1.


TODO constant 0 and 1
 */
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, TryFromPrimitive, Clone)]
#[repr(i32)]
pub(crate) enum GateType {
    // ZERO = 0,
    // NOR = 1,
    // A-and-not-B
    // AANB = 2,
    // NOT B
    // INVB = 3,
    // not-A-and-B?
    // NAAB = 4,
    // NOT A
    // INV = 5,
    XOR = 6,
    // NAND = 7,
    AND = 8,
    // XNOR = 9,
    // BUF = 10,
    // A-or-NOT-B?
    // AONB = 11,
    // BUFB = 12,
    // NOT-A-or-B?
    // NAOB = 13,
    // OR = 14,
    // ONE = 15,
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
#[derive(Debug)]
pub(crate) enum GateInternal {
    Standard {
        r#type: GateType,
        input_a: Option<WireRef>,
        input_b: Option<WireRef>,
    },
    // Constant {
    //     value: bool,
    // },
}

impl GateInternal {
    pub(crate) fn get_type(&self) -> &GateType {
        match self {
            GateInternal::Standard {
                r#type,
                input_a,
                input_b,
            } => r#type,
            // TODO?
            // GateInternal::Constant { value } => todo!(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Gate {
    pub(crate) internal: GateInternal,
    /// Gate's output is in practice a Gate's ID or idx
    pub(crate) output: WireRef,
}
