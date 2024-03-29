//! Explain the following algorithm from the paper <https://www.esat.kuleuven.be/cosic/publications/article-3351.pdf> Algorithm 2 Algorithm Gb(1κ, C) 1: . Label sampling 2: for every circuit input wire j ∈ [n] do 3: Sample Kj 0 , Kj 1 ∈R {0, 1}` uniformly at random 4: Set ej = (Kj 0 , Kj 1 ) . Encoding information 5: Denote by ej [xj ] the value Kj xj 6: end for 7: . Key Extraction (Section 3.1) 8: for each gate g of C in topological order do 9: Compress (f1,0): . See Section 3.1 (Compress) 10: X00 = RO0(KA 0 , KB 0 ) 11: X01 = RO0(KA 0 , KB 1 ) 12: X10 = RO0(KA 1 , KB 0 ) 13: X11 = RO0(KA 1 , KB 1 ) 14: 15: Collapse (f1,1): . See Section 3.1 (Collapse) 16: if g = ∧ then 17: ∇ = ∇∧ . See Table 1 18: else 19: ∇ = ∇⊕ . See Table 2 20: end if 21: S0 = X00&∇ 22: if g = ⊕ then 23: S1 = X01&∇ 24: else (g = ∧) 25: S1 = X11&∇ 26: end if 27: . Output hiding (Section 3.1) 28: KC 0 = RO1(S0) 29: KC 1 = RO1(S1) 30: F [g] ← ∇ 31: end for 32: . Decoding information 33: for For every output wire j ∈ [n + q − m + 1, n + q] do 34: Sample dj ∈R {0, 1}` s.t. lsb(RO2(Kj 0 , dj )) = 0 and lsb(RO2(Kj 1 , dj )) = 1 35: end for 36: Return (F, e, d) where e = {ej } and d = {dj }.
//!
//! Answer
//! The algorithm presented in the paper is a garbling scheme, which is a cryptographic technique used to create a garbled circuit. A garbled circuit is an encrypted version of a Boolean circuit that allows secure computation of the circuit's output without revealing the input values. The goal of the garbling scheme is to allow two parties to compute a function on their private inputs without revealing the inputs to each other.
//! Algorithm 2 (Gb) in the paper defines the garbling procedure for a given circuit C. Here's a high-level explanation of the algorithm:
//! -   Label sampling: For each input wire, sample two random labels (Kj^0 and Kj^1) and set the encoding information (ej)
//!     as a tuple of these labels.
//! -   Key Extraction: For each gate in the circuit, apply the "compress" and "collapse" functions.
//!     The "compress" function (f1,0) computes the XOR of the input labels for each possible input combination (X00, X01, X10, X11).
//!     The "collapse" function (f1,1) computes the output labels (S0 and S1) based on the type of gate (AND or XOR) and the input labels.
//! -   Output hiding: Compute the output labels (KC^0 and KC^1) by applying a random oracle function (RO1)
//!     on the output labels obtained from the "collapse" function.
//! -   Decoding information: For each output wire, sample a random value (dj) such that
//!     the least significant bit (lsb) of the application of another random oracle function (RO2)
//!     on the output labels and dj satisfies a specific condition.
//!
//! The algorithm returns a tuple (F, e, d), where F is the garbled circuit, e is the encoding information for the input wires, and d is the decoding information for the output wires. This tuple is later used in other parts of the garbling scheme to securely compute the circuit's output.
//!
//! It's important to note that the algorithm assumes the existence of random oracle functions (RO0, RO1, and RO2), which are idealized cryptographic hash functions. In practice, these would be replaced with secure hash functions or other cryptographic primitives to achieve the desired security properties.
//!
//! [generated with phind.com (GPT 4?)]

//! 2.1 Preliminaries
//! page 4
//!
//! "Circuit syntax. A Boolean circuit C : {0, 1}n → {0, 1}m has n input wires
//! enumerated by the indices 1, . . . , n, and m output wires enumerated by n + q −
//! m + 1, . . . , n + q, where q = |C| is the number Boolean gates.8 The output wire
//! of gate j (also denoted by gj ) is n + j, which also implies that the gates satisfy
//! a topological order, allowing to speak of gi > gj when i > j. On occasion, we
//! abuse notation and use g as a synonym for the binary function described by this
//! gate. Namely, gj (a, b) is the result of the binary function of gate gj on the binary
//! inputs a and b. For example, if gj is an XOR gate then gj (a, b) = a ⊕ b. The
//! interpretation would always be clear from the context.""

mod block;
mod circuit_for_eval;
mod constant;
mod delta;
mod random_oracle;
mod wire_labels_set;
mod wire_labels_set_bitslice;

pub(crate) mod evaluate;
pub(crate) mod garble;
pub(crate) mod wire;
pub(crate) mod wire_value;

pub(super) use garble::GarblerError;

#[cfg(feature = "key_length_search")]
mod key_length;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::new_garbling_scheme::{evaluate::evaluate_full_chain, garble::garble};
    use circuit_types_rs::{Circuit, KindBinary, KindUnary};

    #[derive(Debug)]
    enum TestGateType {
        Binary(KindBinary),
        Unary(KindUnary),
        Constant(bool),
    }

    /// param `tests`: inputs, `expected_output`
    fn aux_test_basic_circuit(
        tests: Vec<(Vec<wire_value::WireValue>, wire_value::WireValue)>,
        gate_type_to_test: TestGateType,
    ) {
        // to be on the safe side, we run each test multiple times
        // that is b/c we so much random and bit/lsb/msb stuff, we can easily end up
        // with a wrong algo that sometimes return a good output
        for idx in 0..100 {
            for (inputs, expected_output) in tests.clone() {
                let circ = match &gate_type_to_test {
                    TestGateType::Binary(gate_type) => Circuit::new_test_circuit(gate_type.clone()),
                    TestGateType::Unary(gate_type) => {
                        Circuit::new_test_circuit_unary(gate_type.clone())
                    }
                    TestGateType::Constant(value) => Circuit::new_test_circuit_constant(*value),
                };
                let garbled = garble(circ, None).unwrap();

                let outputs = evaluate_full_chain(&garbled, &inputs).unwrap();
                println!("outputs : {outputs:?} [{idx}]");
                assert_eq!(
                    outputs.len(),
                    1,
                    "{gate_type_to_test:?} gate so we SHOULD have only one output!",
                );
                assert_eq!(outputs[0], expected_output);
            }
        }
    }

    #[test]
    fn test_basic_or() {
        // inputs, expected_output
        let tests: Vec<(Vec<wire_value::WireValue>, wire_value::WireValue)> = vec![
            // Standard truth table for OR Gate
            // (input0, input1), output
            (vec![false.into(), false.into()], false.into()),
            (vec![false.into(), true.into()], true.into()),
            (vec![true.into(), false.into()], true.into()),
            (vec![true.into(), true.into()], true.into()),
        ];

        aux_test_basic_circuit(tests, TestGateType::Binary(KindBinary::OR));
    }

    #[test]
    fn test_basic_and() {
        // inputs, expected_output
        let tests: Vec<(Vec<wire_value::WireValue>, wire_value::WireValue)> = vec![
            // Standard truth table for AND Gate
            // (input0, input1), output
            (vec![false.into(), false.into()], false.into()),
            (vec![false.into(), true.into()], false.into()),
            (vec![true.into(), false.into()], false.into()),
            (vec![true.into(), true.into()], true.into()),
        ];

        aux_test_basic_circuit(tests, TestGateType::Binary(KindBinary::AND));
    }

    #[test]
    fn test_basic_xor() {
        // inputs, expected_output
        let tests: Vec<(Vec<wire_value::WireValue>, wire_value::WireValue)> = vec![
            // Standard truth table for XOR Gate
            // (input0, input1), output
            (vec![false.into(), false.into()], false.into()),
            (vec![false.into(), true.into()], true.into()),
            (vec![true.into(), false.into()], true.into()),
            (vec![true.into(), true.into()], false.into()),
        ];

        aux_test_basic_circuit(tests, TestGateType::Binary(KindBinary::XOR));
    }

    #[test]
    fn test_basic_nand() {
        // inputs, expected_output
        let tests: Vec<(Vec<wire_value::WireValue>, wire_value::WireValue)> = vec![
            // Standard truth table for NAND Gate
            // (input0, input1), output
            (vec![false.into(), false.into()], true.into()),
            (vec![false.into(), true.into()], true.into()),
            (vec![true.into(), false.into()], true.into()),
            (vec![true.into(), true.into()], false.into()),
        ];

        aux_test_basic_circuit(tests, TestGateType::Binary(KindBinary::NAND));
    }

    #[test]
    fn test_basic_nor() {
        // inputs, expected_output
        let tests: Vec<(Vec<wire_value::WireValue>, wire_value::WireValue)> = vec![
            // Standard truth table for NOR Gate
            // (input0, input1), output
            (vec![false.into(), false.into()], true.into()),
            (vec![false.into(), true.into()], false.into()),
            (vec![true.into(), false.into()], false.into()),
            (vec![true.into(), true.into()], false.into()),
        ];

        aux_test_basic_circuit(tests, TestGateType::Binary(KindBinary::NOR));
    }

    #[test]
    fn test_basic_xnor() {
        // inputs, expected_output
        let tests: Vec<(Vec<wire_value::WireValue>, wire_value::WireValue)> = vec![
            // Standard truth table for XNOR Gate (also known as XAND)
            // (input0, input1), output
            (vec![false.into(), false.into()], true.into()),
            (vec![false.into(), true.into()], false.into()),
            (vec![true.into(), false.into()], false.into()),
            (vec![true.into(), true.into()], true.into()),
        ];

        aux_test_basic_circuit(tests, TestGateType::Binary(KindBinary::XNOR));
    }

    #[test]
    fn test_basic_not() {
        // inputs, expected_output
        let tests: Vec<(Vec<wire_value::WireValue>, wire_value::WireValue)> = vec![
            // Standard truth table for NOT Gate
            // (input0, input1), output
            (vec![false.into()], true.into()),
            (vec![true.into()], false.into()),
        ];

        aux_test_basic_circuit(tests, TestGateType::Unary(KindUnary::INV));
    }

    #[test]
    fn test_basic_buf() {
        // inputs, expected_output
        let tests: Vec<(Vec<wire_value::WireValue>, wire_value::WireValue)> = vec![
            // Standard truth table for BUF Gate
            // (input0, input1), output
            (vec![false.into()], false.into()),
            (vec![true.into()], true.into()),
        ];

        aux_test_basic_circuit(tests, TestGateType::Unary(KindUnary::BUF));
    }

    #[test]
    // TODO(new-garbling-scheme)[opt-0-1] should probably apply the same "free-BUF" for constant 0/1
    //  right now the 0/1 gates are rewritten by skcd_parser so we can not build a circuit with them directy
    fn test_basic_zero() {
        // inputs, expected_output
        let tests: Vec<(Vec<wire_value::WireValue>, wire_value::WireValue)> = vec![
            // Standard truth table for 0 Gate
            // (input0, input1), output
            (vec![false.into()], false.into()),
            (vec![true.into()], false.into()),
        ];

        aux_test_basic_circuit(tests, TestGateType::Constant(false));
    }

    #[test]
    // TODO(new-garbling-scheme)[opt-0-1] should probably apply the same "free-BUF" for constant 0/1
    //  right now the 0/1 gates are rewritten by skcd_parser so we can not build a circuit with them directy
    fn test_basic_one() {
        // inputs, expected_output
        let tests: Vec<(Vec<wire_value::WireValue>, wire_value::WireValue)> = vec![
            // Standard truth table for 1 Gate
            // (input0, input1), output
            (vec![false.into()], true.into()),
            (vec![true.into()], true.into()),
        ];

        aux_test_basic_circuit(tests, TestGateType::Constant(true));
    }

    #[test]
    fn test_garble_adder() {
        let circ = circuit_types_rs::deserialize_from_buffer(include_bytes!(
            "../../examples/data/result_abc_full_adder.postcard.bin"
        ))
        .unwrap();

        garble(circ, None).unwrap();
    }
}
