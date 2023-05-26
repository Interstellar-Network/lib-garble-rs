//! Explain the following algorithm from the paper https://www.esat.kuleuven.be/cosic/publications/article-3351.pdf Algorithm 2 Algorithm Gb(1κ, C) 1: . Label sampling 2: for every circuit input wire j ∈ [n] do 3: Sample Kj 0 , Kj 1 ∈R {0, 1}` uniformly at random 4: Set ej = (Kj 0 , Kj 1 ) . Encoding information 5: Denote by ej [xj ] the value Kj xj 6: end for 7: . Key Extraction (Section 3.1) 8: for each gate g of C in topological order do 9: Compress (f1,0): . See Section 3.1 (Compress) 10: X00 = RO0(KA 0 , KB 0 ) 11: X01 = RO0(KA 0 , KB 1 ) 12: X10 = RO0(KA 1 , KB 0 ) 13: X11 = RO0(KA 1 , KB 1 ) 14: 15: Collapse (f1,1): . See Section 3.1 (Collapse) 16: if g = ∧ then 17: ∇ = ∇∧ . See Table 1 18: else 19: ∇ = ∇⊕ . See Table 2 20: end if 21: S0 = X00&∇ 22: if g = ⊕ then 23: S1 = X01&∇ 24: else (g = ∧) 25: S1 = X11&∇ 26: end if 27: . Output hiding (Section 3.1) 28: KC 0 = RO1(S0) 29: KC 1 = RO1(S1) 30: F [g] ← ∇ 31: end for 32: . Decoding information 33: for For every output wire j ∈ [n + q − m + 1, n + q] do 34: Sample dj ∈R {0, 1}` s.t. lsb(RO2(Kj 0 , dj )) = 0 and lsb(RO2(Kj 1 , dj )) = 1 35: end for 36: Return (F, e, d) where e = {ej } and d = {dj }.
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

use hashbrown::{hash_map::OccupiedError, HashMap, HashSet};
use serde::{Deserialize, Serialize};

mod block;
mod constant;
mod delta;
mod random_oracle;
mod wire;
mod wire_labels_set;
mod wire_labels_set_bitslice;
mod wire_value;

#[cfg(feature = "key_length_search")]
mod key_length;

use super::GarblerError;
use crate::circuit::{Circuit, Gate, GateType, WireRef};
use block::{BlockL, BlockP};
use random_oracle::RandomOracle;
use wire::{Wire, WireLabel};

/// In https://eprint.iacr.org/2021/739.pdf
/// this is the lines 1 to 4 of "Algorithm 5 Gate"
/// 1: Xg00 = ROg (LA0 , LB0 )
/// 2: Xg01 = ROg (LA0 , LB1 )
/// 3: Xg10 = ROg (LA1 , LB0 )
/// 4: Xg11 = ROg (LA1 , LB1 )
///
/// Also called `Compress` in https://www.esat.kuleuven.be/cosic/publications/article-3351.pdf
/// The function f1,0, which we model as a random oracle, is used to
/// compress each pair into a random string of length `, i.e.,
/// X00 = f1,0(KA0 , KB0 ) = RO0(KA0 , KB0 );
/// X01 = f1,0(KA0 , KB1 ) = RO0(KA0 , KB1 );
/// X10 = f1,0(KA1 , KB0 ) = RO0(KA1 , KB0 );
/// X11 = f1,0(KA1 , KB1 ) = RO0(KA1 , KB1 )."
///
/// parameter:
/// - gate: "The random oracle RO employed throughout the gate-by-gate
/// garbling process is tweakable: it takes as an additional input the gate index g so
/// that it behaves independently for each gate."
///
fn f1_0_compress(w: &InputEncodingSet, gate: &Gate) -> wire_labels_set::WireLabelsSet {
    let tweak = gate.get_id();

    match gate.get_type() {
        GateType::Binary {
            r#type,
            input_a,
            input_b,
        } => {
            let wire_a: &Wire = &w.e[input_a];
            let wire_b: &Wire = &w.e[input_b];

            wire_labels_set::WireLabelsSet::new_binary(
                RandomOracle::random_oracle_g(&wire_a.value0(), Some(&wire_b.value0()), tweak),
                RandomOracle::random_oracle_g(&wire_a.value0(), Some(&wire_b.value1()), tweak),
                RandomOracle::random_oracle_g(&wire_a.value1(), Some(&wire_b.value0()), tweak),
                RandomOracle::random_oracle_g(&wire_a.value1(), Some(&wire_b.value1()), tweak),
            )
        }
        GateType::Unary { r#type, input_a } => {
            let wire_a: &Wire = &w.e[input_a];

            wire_labels_set::WireLabelsSet::new_unary(
                RandomOracle::random_oracle_g(&wire_a.value0(), None, tweak),
                RandomOracle::random_oracle_g(&wire_a.value1(), None, tweak),
            )
        }
    }
}

/// "input encoding set e."
///
/// NOTE: Contrary to the papers it is a HashMap instead of a Vec in topological order
/// b/c in `fn garble` when looping on `circuit.gates` the gate.id is NOT guaranteed to be in order!
/// eg
/// - circuits inputs: *should* indeed usually be in order => for instance 0..2
/// - BUT the first "Gate ID" could be eg 5
/// - which means the second iteration of the loop would not work without a hashmap
///
#[derive(Clone)]
struct InputEncodingSet {
    e: HashMap<WireRef, Wire>,
}

/// Initialize the `W` which is the set of wires:
/// TODO? Does two things:
/// - allocate the full `W` set with the correct number of wires
/// - set the first wires == the input wires to random
///
/// First part of the sequence:
/// (1) Init(C) → e;
/// (2) Circuit(C, e) = (F, D);
/// (3) DecodingInfo(D) → d
///
/// See "Algorithm 4 Circuit" in https://eprint.iacr.org/2021/739.pdf
/// up to 5:
///
/// See also:
/// "Algorithm 3 Init(C, ℓ)" in https://www.esat.kuleuven.be/cosic/publications/article-3351.pdf
///
/// 1: extract n from C and initialize e = []
/// 2:  for input wire W ∈ [n] do
/// 3:      Sample LW0 ← {0, 1}ℓ uniformly at random
/// 4:      Sample LW1 ← {0, 1}ℓ − {LW0 } uniformly at random
/// 5:      Set e[W ] = eW = (LW0 , LW1 )
/// 6:  end for
/// 7: Return e
///
fn init_internal(circuit: &Circuit, random_oracle: &mut RandomOracle) -> InputEncodingSet {
    let mut w = HashMap::with_capacity(circuit.n() as usize);
    for (idx, input_wire) in circuit.wires()[0..circuit.n() as usize].iter().enumerate() {
        // CHECK: the Wires MUST be iterated in topological order!
        assert_eq!(
            input_wire.id, idx,
            "Wires MUST be iterated in topological order!"
        );

        let lw0 = random_oracle.new_random_blockL();
        let lw1 = random_oracle.new_random_blockL();

        // NOTE: if this fails: add a diff(cf pseudocode) or xor or something like that
        assert!(lw0 != lw1, "LW0 and LW1 MUST NOT be the same!");

        w.insert(WireRef { id: input_wire.id }, Wire::new(lw0, lw1));
    }

    assert_eq!(w.len(), circuit.inputs.len(), "wrong w length! [1]");
    assert_eq!(w.len(), circuit.n() as usize, "wrong w length! [2]");

    // w.extend((0..circuit.q()).iter(). )

    // assert_eq!(w.len(), circuit.n() as usize + circuit.q(), "wrong w length! [3]");

    // w

    InputEncodingSet { e: w }
}

/// Noted `d` in the paper
///
struct DecodedInfo {
    d: HashMap<WireRef, BlockL>,
}

/// In https://eprint.iacr.org/2021/739.pdf
/// "Algorithm 6 DecodingInfo(D, ℓ)"
///
/// Last part of the sequence:
/// (1) Init(C) → e;
/// (2) Circuit(C, e) = (F, D);
/// (3) DecodingInfo(D) → d
///
fn decoding_info(
    circuit_outputs: &[WireRef],
    d_up: &D,
    random_oracle: &mut RandomOracle,
) -> DecodedInfo {
    let mut d = HashMap::with_capacity(circuit_outputs.len());

    // "2: for output wire j ∈ [m] do"
    for output_wire in circuit_outputs {
        // "extract Lj0, Lj1 ← D[j]"
        let (lj0, lj1) = d_up.d.get(output_wire).expect("missing output in map!");

        let mut dj = random_oracle.new_random_blockL();
        loop {
            let a = !RandomOracle::random_oracle_prime(lj0, &dj);
            let b = RandomOracle::random_oracle_prime(lj1, &dj);
            if a && b {
                break;
            }
            dj = random_oracle.new_random_blockL();
        }

        d.insert(output_wire.clone(), dj);
    }

    DecodedInfo { d }
}

/// Noted `F` in the paper
struct F {
    /// One per Gate
    f: HashMap<WireRef, delta::Delta>,
}

/// Noted `D` in the paper
struct D {
    d: HashMap<WireRef, (BlockL, BlockL)>,
}

struct GarbledCircuitInternal {
    f: F,
    d: D,
}

/// Garble
///
/// In https://eprint.iacr.org/2021/739.pdf
/// Algorithm 4 Circuit(e, C, ℓ, ℓ′)
///
/// [...]
/// 16: Return (F, D)
///
/// Second part of the sequence:
/// (1) Init(C) → e;
/// (2) Circuit(C, e) = (F, D);
/// (3) DecodingInfo(D) → d
///
fn garble_internal<'a>(
    circuit: &'a Circuit,
    e: &InputEncodingSet,
) -> Result<GarbledCircuitInternal, GarblerError> {
    // "6: initialize F = [], D = []"
    let mut f = HashMap::with_capacity(circuit.gates.len());
    // also noted as: ∇g
    // TODO should this (semantically) be instead `HashMap<&WireRef, Wire>`(or `HashMap<&WireRef, &Wire>`)
    let mut deltas = HashMap::with_capacity(circuit.outputs.len());

    let mut encoded_wires = HashMap::with_capacity(circuit.gates.len());

    let outputs_set: HashSet<&WireRef> = HashSet::from_iter(circuit.outputs.iter());

    for gate in circuit.gates.iter() {
        let compressed_set = f1_0_compress(&e, gate);
        let (l0, l1, delta) = delta::Delta::new(&compressed_set, gate.get_type())?;

        let wire_ref = WireRef { id: gate.get_id() };

        f.try_insert(wire_ref.clone(), delta).unwrap();

        // TODO what index should we use?
        // w is init with [0,n], and as size [0,n+q]
        // what about Gate's index? (== output)
        match encoded_wires.try_insert(wire_ref, Wire::new(l0.into(), l1.into())) {
            Err(OccupiedError { entry, value }) => Err(GarblerError::GateIdOutputMismatch),
            // The key WAS NOT already present; everything is fine
            Ok(wire) => {
                // "12: if g is an output gate then"
                if let Some(wire_output) = outputs_set.get(gate.get_output()) {
                    deltas.insert(
                        wire_output.clone().clone(),
                        (wire.value0().clone(), wire.value1().clone()),
                    );
                }

                Ok(())
            }
        };

        // // let k0 = RandomOracle::random_oracle_1(&s0);
        // // let k1 = RandomOracle::random_oracle_1(&s1);

        // match r#type {
        //     // GateType::INV => todo!(),
        //     GateType::XOR => todo!(),
        //     // GateType::NAND => todo!(),
        //     GateType::AND => todo!(),
        //     // ite = If-Then-Else
        //     // we define BUF as "if input == 1 then input; else 0"
        //     // GateType::BUF => todo!(),
        //     _ => todo!("unsupported gate type! [{:?}]", gate),
        // }
        // TODO?
        // GateInternal::Constant { value } => todo!(),
    }

    // println!("garble_circuit: deltas: {deltas:?}");

    Ok(GarbledCircuitInternal {
        f: F { f },
        d: D { d: deltas },
    })
}

/// This is the EVALUABLE GarbledCircuit; ie the result of the whole garbling pipeline.
pub(crate) struct GarbledCircuitFinal {
    circuit: Circuit,
    garbled_circuit: GarbledCircuitInternal,
    d: DecodedInfo,
    e: InputEncodingSet,
}

/// Grouping of all of the sequence:
/// (1) Init(C) → e;
/// (2) Circuit(C, e) = (F, D);
/// (3) DecodingInfo(D) → d
///
// TODO? how to group the garble part vs eval vs decoding?
pub(crate) fn garble(circuit: Circuit) -> Result<GarbledCircuitFinal, GarblerError> {
    let mut random_oracle = RandomOracle::new();

    let mut e = init_internal(&circuit, &mut random_oracle);

    let garbled_circuit = garble_internal(&circuit, &mut e)?;

    let d = decoding_info(&circuit.outputs, &garbled_circuit.d, &mut random_oracle);

    Ok(GarbledCircuitFinal {
        circuit,
        garbled_circuit,
        d,
        e,
    })
}

/// Noted `X`
///
/// For each Circuit.inputs this will be a `Block` referencing either `value0` or `value1`
///
struct EncodedInfo {
    x: HashMap<WireRef, WireLabel>,
}

/// Encoding
///
/// In https://eprint.iacr.org/2021/739.pdf "Algorithm 7"
///
/// 1: procedure En(e, x)
/// 2: initialize X = []
/// 3: for every j ∈ [n] do
/// 4:  set X[j] = Ljxj = ej [xj ]
/// 5: end for
/// 6: Return X
/// 7: end procedure
///
/// "En(e, x) := X: returns the encoding X for function input x"
///
/// In https://www.esat.kuleuven.be/cosic/publications/article-3351.pdf
/// Algorithm 4 Algorithm En(e, x)
///
/// 1: for every j ∈ [n] do
/// 2:  output Kjxj = ej [xj ]
/// 3: end for
fn encoding_internal<'a>(
    circuit: &'a Circuit,
    e: &'a InputEncodingSet,
    x: &'a [wire_value::WireValue],
) -> EncodedInfo {
    // CHECK: we SHOULD have one "user input" for each Circuit's input(ie == `circuit.n`)
    assert_eq!(
        e.e.len(),
        x.len(),
        "encoding: `x` inputs len MUST match the Circuit's inputs len!"
    );

    let mut x_up = EncodedInfo {
        x: HashMap::with_capacity(x.len()),
    };

    for (input_wire, input_value) in circuit.inputs.iter().zip(x) {
        let encoded_wire = e.e.get(input_wire).unwrap();
        let block = if input_value.value {
            encoded_wire.value1()
        } else {
            encoded_wire.value0()
        };
        x_up.x.insert(input_wire.clone(), WireLabel::new(block));
    }

    assert_eq!(x_up.x.len(), e.e.len(), "EncodedInfo: wrong length!");
    x_up
}

/// Noted `Y` in the paper
struct OutputLabels {
    y: HashMap<WireRef, BlockL>,
}

///
/// In Algorithm 7 "Algorithms to Evaluate the Garbling"
/// 9: procedure Ev(F, X)
/// [...]
/// 18: Return Y
/// 19: end procedure
///
/// "Ev(F, X) := Y : returns the output labels Y by evaluating F on X."
///
fn evaluate_internal(circuit: &Circuit, f: &F, encoded_info: &EncodedInfo) -> OutputLabels {
    let mut output_labels = OutputLabels {
        y: HashMap::with_capacity(circuit.outputs.len()),
    };

    let outputs_set: HashSet<&WireRef> = HashSet::from_iter(circuit.outputs.iter());

    // As we are looping on the gates in order, this will be built step by step
    // ie the first gates are inputs, and this will already contain them
    // them we built all the other gates in subsequent iterations of the loop
    let active_wires = encoded_info.x.clone();

    // "for each gate g ∈ [q] in a topological order do"
    for gate in circuit.gates.iter() {
        // "LA, LB ← active labels associated with the input wires of gate g"
        let (l_a, l_b) = match gate.get_type() {
            GateType::Binary {
                r#type,
                input_a,
                input_b,
            } => {
                let l_a = active_wires.get(input_a).unwrap();
                let l_b = active_wires.get(input_b).unwrap();

                (l_a.get_block(), Some(l_b.get_block()))
            }
            GateType::Unary { r#type, input_a } => {
                let l_a = active_wires.get(input_a).unwrap();
                (l_a.get_block(), None)
            }
        };

        let wire_ref = WireRef { id: gate.get_id() };

        // "extract ∇g ← F [g]"
        let delta_g = f.f.get(&wire_ref).unwrap();

        // "compute Lg ← RO(g, LA, LB ) ◦ ∇g"
        let r = RandomOracle::random_oracle_g(l_a, l_b, gate.get_id());
        let l_g_full = BlockP::new_projection(&r, delta_g.get_block());
        let l_g: BlockL = l_g_full.into();

        // "if g is a circuit output wire then"
        // TODO move the previous lines under the if; or better: iter only on output gates? (filter? or circuit.outputs?)
        if let Some(wire_output) = outputs_set.get(&wire_ref) {
            // "Y [g] ← Lg"
            match output_labels.y.try_insert(wire_ref, l_g) {
                Err(OccupiedError { entry, value }) => Err(GarblerError::EvaluateDuplicatedWire),
                // The key WAS NOT already present; everything is fine
                Ok(wire) => Ok(()),
            };
        }
    }

    output_labels
}

///
/// In Algorithm 7 "Algorithms to Evaluate the Garbling"
/// 21: procedure De(Y, d)
/// [...]
/// 26: Return y
/// 27: end procedure
///
/// "De(Y, d) := {⊥, y}: returns either the failure symbol ⊥ or a value y = f (x)."
///
fn decoding_internal(
    circuit: &Circuit,
    output_labels: &OutputLabels,
    decoded_info: &DecodedInfo,
) -> Vec<wire_value::WireValue> {
    let mut outputs = vec![];

    // "for j ∈ [m] do"
    for output in circuit.outputs.iter() {
        // "y[j] ← lsb(RO′(Y [j], dj ))"
        let yj = output_labels.y.get(output).unwrap();
        let dj = decoded_info.d.get(output).unwrap();
        let r = RandomOracle::random_oracle_prime(yj, dj);
        // NOTE: `random_oracle_prime` directly get the LSB so no need to do it here
        outputs.push(wire_value::WireValue { value: r });
    }

    outputs
}

pub(crate) fn evaluate(
    garbled: &GarbledCircuitFinal,
    x: &[wire_value::WireValue],
) -> Vec<wire_value::WireValue> {
    let encoded_info = encoding_internal(&garbled.circuit, &garbled.e, x);

    let output_labels =
        evaluate_internal(&garbled.circuit, &garbled.garbled_circuit.f, &encoded_info);

    decoding_internal(&garbled.circuit, &output_labels, &garbled.d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{circuit::SkcdConfig, garble::InterstellarCircuit};

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

        for (inputs, expected_output) in tests {
            let circ = InterstellarCircuit::new_test_circuit(crate::circuit::GateTypeBinary::OR);
            let garbled = garble(circ.circuit).unwrap();

            let outputs = evaluate(&garbled, &inputs);
            println!("outputs : {outputs:?}");
            assert_eq!(
                outputs.len(),
                1,
                "OR gate so we SHOULD have only one output!"
            );
            assert_eq!(outputs[0], expected_output);
        }
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

        for (inputs, expected_output) in tests {
            let circ = InterstellarCircuit::new_test_circuit(crate::circuit::GateTypeBinary::AND);
            let garbled = garble(circ.circuit).unwrap();

            let outputs = evaluate(&garbled, &inputs);
            println!("outputs : {outputs:?}");
            assert_eq!(
                outputs.len(),
                1,
                "AND gate so we SHOULD have only one output!"
            );
            assert_eq!(outputs[0], expected_output);
        }
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

        for (inputs, expected_output) in tests {
            let circ = InterstellarCircuit::new_test_circuit(crate::circuit::GateTypeBinary::XOR);
            let garbled = garble(circ.circuit).unwrap();

            let outputs = evaluate(&garbled, &inputs);
            println!("outputs : {outputs:?}");
            assert_eq!(
                outputs.len(),
                1,
                "XOR gate so we SHOULD have only one output!"
            );
            assert_eq!(outputs[0], expected_output);
        }
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

        for (inputs, expected_output) in tests {
            let circ = InterstellarCircuit::new_test_circuit(crate::circuit::GateTypeBinary::NAND);
            let garbled = garble(circ.circuit).unwrap();

            let outputs = evaluate(&garbled, &inputs);
            println!("outputs : {outputs:?}");
            assert_eq!(
                outputs.len(),
                1,
                "NAND gate so we SHOULD have only one output!"
            );
            assert_eq!(outputs[0], expected_output);
        }
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

        for (inputs, expected_output) in tests {
            let circ = InterstellarCircuit::new_test_circuit(crate::circuit::GateTypeBinary::NOR);
            let garbled = garble(circ.circuit).unwrap();

            let outputs = evaluate(&garbled, &inputs);
            println!("outputs : {outputs:?}");
            assert_eq!(
                outputs.len(),
                1,
                "NOR gate so we SHOULD have only one output!"
            );
            assert_eq!(outputs[0], expected_output);
        }
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

        for (inputs, expected_output) in tests {
            let circ =
                InterstellarCircuit::new_test_circuit_unary(crate::circuit::GateTypeUnary::INV);
            let garbled = garble(circ.circuit).unwrap();

            let outputs = evaluate(&garbled, &inputs);
            assert_eq!(
                outputs.len(),
                1,
                "NOT gate so we SHOULD have only one output!"
            );
            assert_eq!(outputs[0], expected_output);
        }
    }

    #[test]
    fn test_garble() {
        let circ = InterstellarCircuit::parse_skcd(include_bytes!(
            "../../../examples/data/adder.skcd.pb.bin"
        ))
        .unwrap();

        garble(circ.circuit);
    }

    #[test]
    fn test_decoding_info() {
        let circuit_outputs = vec![WireRef { id: 42 }];
        let mut random_oracle = RandomOracle::new();
        let mut d_up = HashMap::new();
        let l0 = random_oracle.new_random_blockL();
        let l1 = random_oracle.new_random_blockL();
        d_up.insert(circuit_outputs[0].clone(), (l0.clone(), l1.clone()));

        let d = D { d: d_up };

        let d = decoding_info(&circuit_outputs, &d, &mut random_oracle);
        let dj = &d.d.get(&circuit_outputs[0]).unwrap();
        assert_eq!(RandomOracle::random_oracle_prime(&l0, dj), false);
        assert_eq!(RandomOracle::random_oracle_prime(&l1, dj), true);
    }
}
