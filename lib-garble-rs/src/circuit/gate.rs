use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

use crate::skcd_parser::CircuitParserError;

// derive_partial_eq_without_eq: https://github.com/neoeinstein/protoc-gen-prost/issues/26
#[allow(clippy::derive_partial_eq_without_eq)]
#[allow(clippy::perf)]
#[allow(clippy::pedantic)]
mod interstellarpbskcd {
    // TODO(interstellar) can we use prost-build(and prost-derive) in SGX env?
    // include!(concat!(env!("OUT_DIR"), "/interstellarpbskcd.rs"));
    include!("../../deps/protos/generated/rust/interstellarpbskcd.rs");
}

/// This is a "reference" to either:
/// - another Gate's inputs
/// - a Gate's output
/// - a Circuit's output
// TODO ideally this SHOULD NOT be cloneable; and we should replace internal `id: usize` by eg `&Wire`
#[derive(Debug, Clone, PartialEq, Hash, Eq, Serialize, Deserialize)]
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
#[derive(Debug, TryFromPrimitive, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub(crate) enum GateTypeBinary {
    // ZERO = 0,
    NOR = 1,
    // A-and-not-B
    // AANB = 2,
    // not-A-and-B?
    // NAAB = 4,
    XOR = 6,
    NAND = 7,
    AND = 8,
    XNOR = 9,
    // BUF = 10,
    // A-or-NOT-B?
    // AONB = 11,
    // BUFB = 12,
    // NOT-A-or-B?
    // NAOB = 13,
    OR = 14,
    // ONE = 15,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, TryFromPrimitive, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub(crate) enum GateTypeUnary {
    // NOT B
    // INVB = 3,
    // NOT A
    INV = 5,
    BUF = 10,
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
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub(crate) enum GateType {
    Binary {
        gate_type: GateTypeBinary,
        input_a: WireRef,
        input_b: WireRef,
    },
    Unary {
        gate_type: GateTypeUnary,
        input_a: WireRef,
    },
    /// Constant gates (ie 0 and 1) are a special case wrt to parsing the .skcd and garbling/evaluating:
    /// they are "rewritten" using AUX Gate (eg XOR(A,A) = 0, XNOR(A,A) = 1)
    /// That is because contrary to Unary gates, the paper does not explain how to
    /// generalize "Garbling other gate functionalities" to 0 input gate.
    Constant { value: bool },
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub(crate) struct Gate {
    pub(super) internal: GateType,
    /// Gate's output is in practice a Gate's ID or idx
    pub(super) output: WireRef,
}

impl Gate {
    /// Called by `skcd_parser.rs`: build a new Gate based on a given `i32`
    /// which is a Protobuf `interstellarpbskcd::SkcdGateType`
    pub(crate) fn new_from_skcd_gate_type(
        skcd_gate_type_i32: i32,
        output: &WireRef,
        input_a: Option<&WireRef>,
        input_b: Option<&WireRef>,
    ) -> Result<Self, CircuitParserError> {
        let skcd_gate_type_res = interstellarpbskcd::SkcdGateType::from_i32(skcd_gate_type_i32);

        let internal = match skcd_gate_type_res {
            Some(skcd_gate_type) => match skcd_gate_type {
                interstellarpbskcd::SkcdGateType::Inv => Ok(GateType::Unary {
                    gate_type: GateTypeUnary::INV,
                    input_a: input_a.unwrap().clone(),
                }),
                interstellarpbskcd::SkcdGateType::Buf => Ok(GateType::Unary {
                    gate_type: GateTypeUnary::BUF,
                    input_a: input_a.unwrap().clone(),
                }),
                interstellarpbskcd::SkcdGateType::Xor => Ok(GateType::Binary {
                    gate_type: GateTypeBinary::XOR,
                    input_a: input_a.unwrap().clone(),
                    input_b: input_b.unwrap().clone(),
                }),
                interstellarpbskcd::SkcdGateType::Nand => Ok(GateType::Binary {
                    gate_type: GateTypeBinary::NAND,
                    input_a: input_a.unwrap().clone(),
                    input_b: input_b.unwrap().clone(),
                }),
                interstellarpbskcd::SkcdGateType::And => Ok(GateType::Binary {
                    gate_type: GateTypeBinary::AND,
                    input_a: input_a.unwrap().clone(),
                    input_b: input_b.unwrap().clone(),
                }),
                interstellarpbskcd::SkcdGateType::Or => Ok(GateType::Binary {
                    gate_type: GateTypeBinary::OR,
                    input_a: input_a.unwrap().clone(),
                    input_b: input_b.unwrap().clone(),
                }),
                interstellarpbskcd::SkcdGateType::Nor => Ok(GateType::Binary {
                    gate_type: GateTypeBinary::NOR,
                    input_a: input_a.unwrap().clone(),
                    input_b: input_b.unwrap().clone(),
                }),
                interstellarpbskcd::SkcdGateType::Xnor => Ok(GateType::Binary {
                    gate_type: GateTypeBinary::XNOR,
                    input_a: input_a.unwrap().clone(),
                    input_b: input_b.unwrap().clone(),
                }),
                // [constant gate special case] ZERO gate are rewritten as XOR(A,A) = 0
                interstellarpbskcd::SkcdGateType::Zero => Ok(GateType::Binary {
                    gate_type: GateTypeBinary::XOR,
                    input_a: input_a.unwrap().clone(),
                    input_b: input_a.unwrap().clone(),
                }),
                // [constant gate special case] ONE gate are rewritten as XNOR(A,A) = 1
                interstellarpbskcd::SkcdGateType::One => Ok(GateType::Binary {
                    gate_type: GateTypeBinary::XNOR,
                    input_a: input_a.unwrap().clone(),
                    input_b: input_a.unwrap().clone(),
                }),
                interstellarpbskcd::SkcdGateType::One => unimplemented!("ONE constant gate"),
                _ => Err(CircuitParserError::UnknownGateType {
                    gate_type: skcd_gate_type_i32,
                }),
            },
            None => Err(CircuitParserError::UnknownGateType {
                gate_type: skcd_gate_type_i32,
            }),
        }?;

        Ok(Self {
            internal,
            output: output.clone(),
        })
    }

    // TODO move to `impl Gate` directly; and remove `GateInternal`?
    pub(crate) fn get_type(&self) -> &GateType {
        &self.internal
    }

    pub(crate) fn get_id(&self) -> usize {
        self.output.id
    }

    pub(crate) fn get_output(&self) -> &WireRef {
        &self.output
    }
}
