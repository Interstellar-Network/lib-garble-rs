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

use crate::circuit::{Circuit, Gate, GateType, WireRef};

mod block;
mod constant;
mod delta;
mod random_oracle;

use block::{BlockL, BlockP};
use random_oracle::RandomOracle;

use super::GarblerError;

/// Represent a Wire's value, so essentially ON/OFF <=> a boolean
#[repr(transparent)]
#[derive(PartialEq, Debug, Serialize, Deserialize, Default, Clone)]
pub(super) struct WireValue {
    value: bool,
}

impl PartialEq<bool> for WireValue {
    fn eq(&self, other: &bool) -> bool {
        &self.value == other
    }
}

impl PartialEq<bool> for &WireValue {
    fn eq(&self, other: &bool) -> bool {
        &self.value == other
    }
}

impl From<bool> for WireValue {
    fn from(value: bool) -> Self {
        Self { value }
    }
}

/// "Collectively, the set of labels associated with the wire is denoted by {Kj}"
struct K_label {
    value0: BlockL,
    value1: BlockL,
}

/// "The Label Sampling Function f0 This function assigns an l-bit label Kj to
/// each possible value that wire j can take. Collectively, the set of labels associated
/// with the wire is denoted by {Kj }. In particular, Yao’s scheme and all subsequent
/// optimizations decompose the circuit’s input into bits and each bit is assigned a
/// label (See also [App17]).""
// fn f0_label_sampling(wire: Wire) -> K_labels_set {
//     K_labels_set {
//         value0: Block::random(),
//         value1: Block::random(),
//     }
// }

#[derive(Debug, PartialEq, Clone)]
enum CompressedSetInternal {
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

struct CompressedSet {
    internal: CompressedSetInternal,
}

impl CompressedSet {
    pub(crate) fn new_binary(x00: BlockP, x01: BlockP, x10: BlockP, x11: BlockP) -> Self {
        Self {
            internal: CompressedSetInternal::BinaryGate { x00, x01, x10, x11 },
        }
    }

    pub(crate) fn new_unary(x0: BlockP, x1: BlockP) -> Self {
        Self {
            internal: CompressedSetInternal::UnaryGate { x0, x1 },
        }
    }

    /// In https://eprint.iacr.org/2021/739.pdf this is a helper for
    /// "Algorithm 5 Gate"
    /// 7: Set slice ← Xg00[j]||Xg01[j]||Xg10[j]||Xg11[j]
    ///
    /// Return the specific BIT for each x00,x01,x10,x11
    pub(super) fn get_bits_slice(&self, index: usize) -> CompressedSetBitSlice {
        match &self.internal {
            CompressedSetInternal::BinaryGate { x00, x01, x10, x11 } => CompressedSetBitSlice {
                internal: CompressedSetBitSliceInternal::BinaryGate {
                    x00: x00.get_bit(index),
                    x01: x01.get_bit(index),
                    x10: x10.get_bit(index),
                    x11: x11.get_bit(index),
                },
            },
            CompressedSetInternal::UnaryGate { x0, x1 } => CompressedSetBitSlice {
                internal: CompressedSetBitSliceInternal::UnaryGate {
                    x0: x0.get_bit(index),
                    x1: x1.get_bit(index),
                },
            },
        }
    }

    pub(super) fn get_x00(&self) -> &BlockP {
        match &self.internal {
            CompressedSetInternal::BinaryGate { x00, x01, x10, x11 } => x00,
            CompressedSetInternal::UnaryGate { x0, x1 } => {
                unimplemented!("CompressedSetInternal::UnaryGate")
            }
        }
    }

    pub(super) fn get_x01(&self) -> &BlockP {
        match &self.internal {
            CompressedSetInternal::BinaryGate { x00, x01, x10, x11 } => x01,
            CompressedSetInternal::UnaryGate { x0, x1 } => {
                unimplemented!("CompressedSetInternal::UnaryGate")
            }
        }
    }

    pub(super) fn get_x10(&self) -> &BlockP {
        match &self.internal {
            CompressedSetInternal::BinaryGate { x00, x01, x10, x11 } => x10,
            CompressedSetInternal::UnaryGate { x0, x1 } => {
                unimplemented!("CompressedSetInternal::UnaryGate")
            }
        }
    }

    pub(super) fn get_x11(&self) -> &BlockP {
        match &self.internal {
            CompressedSetInternal::BinaryGate { x00, x01, x10, x11 } => x11,
            CompressedSetInternal::UnaryGate { x0, x1 } => {
                unimplemented!("CompressedSetInternal::UnaryGate")
            }
        }
    }

    pub(super) fn get_x0(&self) -> &BlockP {
        match &self.internal {
            CompressedSetInternal::BinaryGate { x00, x01, x10, x11 } => {
                unimplemented!("CompressedSetInternal::BinaryGate")
            }
            CompressedSetInternal::UnaryGate { x0, x1 } => x0,
        }
    }

    pub(super) fn get_x1(&self) -> &BlockP {
        match &self.internal {
            CompressedSetInternal::BinaryGate { x00, x01, x10, x11 } => {
                unimplemented!("CompressedSetInternal::BinaryGate")
            }
            CompressedSetInternal::UnaryGate { x0, x1 } => x1,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(super) enum CompressedSetBitSliceInternal {
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

#[derive(Debug, PartialEq, Clone)]
struct CompressedSetBitSlice {
    internal: CompressedSetBitSliceInternal,
}

impl CompressedSetBitSlice {
    pub(super) fn new_binary_gate_from_bool(x00: bool, x01: bool, x10: bool, x11: bool) -> Self {
        Self {
            internal: CompressedSetBitSliceInternal::BinaryGate {
                x00: x00.into(),
                x01: x01.into(),
                x10: x10.into(),
                x11: x11.into(),
            },
        }
    }

    pub(super) fn new_unary_gate_from_bool(x0: bool, x1: bool) -> Self {
        Self {
            internal: CompressedSetBitSliceInternal::UnaryGate {
                x0: x0.into(),
                x1: x1.into(),
            },
        }
    }
}

impl PartialEq<[bool; 4]> for CompressedSetBitSlice {
    fn eq(&self, other: &[bool; 4]) -> bool {
        match &self.internal {
            CompressedSetBitSliceInternal::BinaryGate { x00, x01, x10, x11 } => {
                x00 == other[0] && x01 == other[1] && x10 == other[2] && x11 == other[3]
            }
            CompressedSetBitSliceInternal::UnaryGate { x0, x1 } => {
                unimplemented!("PartialEq<[bool; 4]> for UnaryGate")
            }
        }
    }
}

impl PartialEq<[bool; 2]> for CompressedSetBitSlice {
    fn eq(&self, other: &[bool; 2]) -> bool {
        match &self.internal {
            CompressedSetBitSliceInternal::BinaryGate { x00, x01, x10, x11 } => {
                unimplemented!("PartialEq<[bool; 4]> for BinaryGate")
            }
            CompressedSetBitSliceInternal::UnaryGate { x0, x1 } => x0 == other[2] && x1 == other[1],
        }
    }
}

impl PartialEq<[WireValue; 4]> for CompressedSetBitSlice {
    fn eq(&self, other: &[WireValue; 4]) -> bool {
        match &self.internal {
            CompressedSetBitSliceInternal::BinaryGate { x00, x01, x10, x11 } => {
                x00 == &other[0] && x01 == &other[1] && x10 == &other[2] && x11 == &other[3]
            }
            CompressedSetBitSliceInternal::UnaryGate { x0, x1 } => {
                unimplemented!("PartialEq<[WireValue; 4]> for UnaryGate")
            }
        }
    }
}

impl PartialEq<[WireValue; 2]> for CompressedSetBitSlice {
    fn eq(&self, other: &[WireValue; 2]) -> bool {
        match &self.internal {
            CompressedSetBitSliceInternal::BinaryGate { x00, x01, x10, x11 } => {
                unimplemented!("PartialEq<[WireValue; 4]> for BinaryGate")
            }
            CompressedSetBitSliceInternal::UnaryGate { x0, x1 } => {
                x0 == &other[2] && x1 == &other[1]
            }
        }
    }
}

/// How to implement the "compress" function ("f1,0" in the papers)?
/// Rust implementation of "compress" function ("f1,0") using ChaCha20Rng and rand crate
/// Answer
/// To implement the "compress" function (f1,0 in the papers), you can use a hash function that compresses its input into a fixed-size output. In this case, let's use the blake2 crate for the Blake2b hash function.

/// "3.1 Garbling
/// Key Extraction Similarly to the classical Yao’s garbled circuit, f1 first splits
/// the four inputs, namely KA0 , KA1 , KB0 , KB1 coming out from f0, into the pairs:
/// (KA0, KB0), (KA0, KB1), (KA1, KB0), (KA1, KB1).
///
/// Compress.
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
fn f1_0_compress(w: &HashMap<usize, K_label>, gate: &Gate) -> CompressedSet {
    let tweak = gate.get_id();

    match gate.get_type() {
        GateType::Binary {
            r#type,
            input_a,
            input_b,
        } => {
            let wire_a: &K_label = &w[&input_a.id];
            let wire_b: &K_label = &w[&input_b.id];

            CompressedSet::new_binary(
                RandomOracle::random_oracle_g(&wire_a.value0, Some(&wire_b.value0), tweak),
                RandomOracle::random_oracle_g(&wire_a.value0, Some(&wire_b.value1), tweak),
                RandomOracle::random_oracle_g(&wire_a.value1, Some(&wire_b.value0), tweak),
                RandomOracle::random_oracle_g(&wire_a.value1, Some(&wire_b.value1), tweak),
            )
        }
        GateType::Unary { r#type, input_a } => {
            let wire_a: &K_label = &w[&input_a.id];

            CompressedSet::new_unary(
                RandomOracle::random_oracle_g(&wire_a.value0, None, tweak),
                RandomOracle::random_oracle_g(&wire_a.value1, None, tweak),
            )
        }
    }
}

/// Initialize the `W` which is the set of wires:
/// TODO? Does two things:
/// - allocate the full `W` set with the correct number of wires
/// - set the first wires == the input wires to random
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
/// return: contrary to the papers it returns a HashMap instead of a Vec in topological order
/// b/c in `fn garble` when looping on `circuit.gates` the gate.id is NOT guaranteed to be in order!
/// eg
/// - circuits inputs: *should* indeed usually be in order => for instance 0..2
/// - BUT the first "Gate ID" could be eg 5
/// - which means the second iteration of the loop would not work without a hashmap
///
fn init_w(circuit: &Circuit, random_oracle: &mut RandomOracle) -> HashMap<usize, K_label> {
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

        w.insert(
            input_wire.id,
            K_label {
                value0: lw0,
                value1: lw1,
            },
        );
    }

    assert_eq!(w.len(), circuit.inputs.len(), "wrong w length! [1]");
    assert_eq!(w.len(), circuit.n() as usize, "wrong w length! [2]");

    // w.extend((0..circuit.q()).iter(). )

    // assert_eq!(w.len(), circuit.n() as usize + circuit.q(), "wrong w length! [3]");

    // w

    w
}

struct DecodedInfo {
    d: Vec<BlockL>,
}

/// In https://eprint.iacr.org/2021/739.pdf
/// "Algorithm 6 DecodingInfo(D, ℓ)"
fn decoding_info(
    circuit_outputs: &[WireRef],
    d_up: HashMap<&WireRef, (BlockL, BlockL)>,
    random_oracle: &mut RandomOracle,
) -> DecodedInfo {
    let mut d = vec![];

    // "2: for output wire j ∈ [m] do"
    for output_wire in circuit_outputs {
        // "extract Lj0, Lj1 ← D[j]"
        let (lj0, lj1) = d_up.get(output_wire).expect("missing output in map!");

        let mut dj = random_oracle.new_random_blockL();
        loop {
            if !random_oracle.random_oracle_prime(lj0, &dj)
                && random_oracle.random_oracle_prime(lj1, &dj)
            {
                break;
            }
            dj = random_oracle.new_random_blockL();
        }

        d.push(dj);
    }

    DecodedInfo { d }
}

/// Noted `F` in the paper
struct F {
    /// One per Gate
    f: Vec<delta::Delta>,
}

struct GarbledCircuit<'a> {
    f: F,
    /// Noted `D` in the paper
    deltas: HashMap<&'a Gate, (BlockL, BlockL)>,
    /// For now at least we transfer ownership of the underlying clear Circuit
    /// b/c deltas's keys(ie Gate) are reference to the Circuit's
    circuit: &'a Circuit,
}

/// Garble
///
/// In https://eprint.iacr.org/2021/739.pdf
/// Algorithm 4 Circuit(e, C, ℓ, ℓ′)
///
/// [...]
/// 16: Return (F, D)
///
fn garble_circuit<'a>(circuit: &'a Circuit) -> Result<GarbledCircuit<'a>, GarblerError> {
    let mut random_oracle = RandomOracle::new();

    let mut w = init_w(&circuit, &mut random_oracle);

    // "6: initialize F = [], D = []"
    let mut f: Vec<delta::Delta> = Vec::with_capacity(circuit.gates.len());
    // also noted as: ∇g
    let mut deltas: HashMap<&Gate, (BlockL, BlockL)> =
        HashMap::with_capacity(circuit.outputs.len());

    let outputs_set: HashSet<&WireRef> = HashSet::from_iter(circuit.outputs.iter());

    for gate in circuit.gates.iter() {
        let compressed_set = f1_0_compress(&w, gate);
        let (l0, l1, delta) = delta::Delta::new(&compressed_set, gate.get_type());

        f.push(delta);

        // TODO what index should we use?
        // w is init with [0,n], and as size [0,n+q]
        // what about Gate's index? (== output)
        match w.try_insert(
            gate.get_id(),
            K_label {
                value0: l0.clone().into(),
                value1: l1.clone().into(),
            },
        ) {
            Err(OccupiedError { entry, value }) => Err(GarblerError::GateIdOutputMismatch),
            // The key WAS NOT already present; everything is fine
            _ => Ok(()),
        };

        // "12: if g is an output gate then"
        if outputs_set.contains(gate.get_output()) {
            deltas.insert(gate, (l0.into(), l1.into()));
        }

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

    Ok(GarbledCircuit {
        f: F { f },
        deltas,
        circuit,
    })
}

// TODO? how to group the garble part vs eval vs decoding?
pub(crate) fn garble(circuit: Circuit) {
    garble_circuit(&circuit);

    todo!("garble")
}

/// Noted `X`
struct EncodedInfo {
    x: HashMap<usize, K_label>,
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
///
/// In https://www.esat.kuleuven.be/cosic/publications/article-3351.pdf
/// Algorithm 4 Algorithm En(e, x)
///
/// 1: for every j ∈ [n] do
/// 2:  output Kjxj = ej [xj ]
/// 3: end for
fn encoding(circuit: &Circuit) -> EncodedInfo {
    todo!()
}

/// Noted `Y` in the paper
struct EvaluateResult {}

///
/// In Algorithm 7 "Algorithms to Evaluate the Garbling"
/// 9: procedure Ev(F, X)
/// [...]
/// 18: Return Y
/// 19: end procedure
///
fn evaluate(f: &F, encoded_info: &EncodedInfo) -> EvaluateResult {
    todo!()
}

///
/// In Algorithm 7 "Algorithms to Evaluate the Garbling"
/// 21: procedure De(Y, d)
/// [...]
/// 26: Return y
/// 27: end procedure
///
fn decoding(evaluate_result: &EvaluateResult, decoded_info: DecodedInfo) {
    todo!()
}

////////////////////////////////////////////////////////////////////////////////

/// "A Key Length Search" [rug version]
/// Ported from matlab to Rust using phind.com
// pub(crate) fn key_length_search_rug() {
//     use rug::ops::Pow;
//     use rug::Float;
//     use rug::Integer;

//     // Set precision
//     let prec = 1000;

//     // Constants
//     let sigma: f64 = 80.0;
//     let kappa: f64 = 256.0;
//     let search_from: u32 = 1700;
//     let search_to: u32 = 1800;

//     // Variables
//     let mpfsigma = Float::with_val(prec, sigma);
//     let mpfkappa = Float::with_val(prec, kappa);
//     let mpf1 = Float::with_val(prec, 1);
//     let mpf3 = Float::with_val(prec, 3);
//     let mpf4 = Float::with_val(prec, 4);
//     let mpf025 = &mpf1 / &mpf4;
//     let mpf075 = &mpf3 / &mpf4;

//     let mpfnegl = &mpf1 / Float::with_val(prec, Integer::from(2).pow(sigma as u32));

//     // Main loop
//     for ell in search_from..search_to {
//         let mpfl = Float::with_val(prec, ell);
//         let mut mpfbadprob = Float::with_val(prec, 0);

//         for i in 0..(kappa as u32 - 1) {
//             let mpfi = Float::with_val(prec, i);
//             let bin_coeff = Float::with_val(prec, binomial(&mpfl, &mpfi));
//             let term1 = mpf025.clone().pow(mpfi.to_u32().unwrap());
//             let term2 = mpf075.clone().pow((mpfl - mpfi).to_u32().unwrap());
//             mpfbadprob += bin_coeff * term1 * term2;
//         }

//         let log_mpfbadprob = mpfbadprob.log2();
//         println!("ell = {}, mpfbadprob = 2^{}", ell, log_mpfbadprob);

//         if mpfbadprob <= mpfnegl {
//             println!("found ell = {}", ell);
//             break;
//         }
//     }
// }

// fn binomial_rug(n: &rug::Float, k: &rug::Float) -> rug::Float {
//     use rug::Float;

//     let mut res = Float::with_val(n.prec(), 1);

//     for i in 1..=k.to_u32().unwrap() {
//         res *= n - Float::with_val(n.prec(), i - 1);
//         res /= Float::with_val(n.prec(), i);
//     }

//     res
// }

/// "A Key Length Search" [num-bigint+num-traits version]
/// Ported from matlab to Rust using phind.com
#[cfg(feature = "key_length_search")]
pub(crate) fn key_length_search_num(search_from: u32, search_to: u32) -> Option<u32> {
    use num_bigint::BigInt;
    use num_traits::identities::One;
    use num_traits::identities::Zero;

    // Constants
    let sigma: u32 = 80;
    let kappa: u32 = 256;

    // Variables
    let negl = BigInt::from(2).pow(sigma) / 2;

    // Main loop
    let mut ell: Option<u32> = None;
    for cur_ell in search_from..search_to {
        let mut badprob = BigInt::zero();

        for i in 0..kappa {
            let bin_coeff = binomial_num(cur_ell, i);
            let term1 = BigInt::from(2).pow(i);
            let term2 = BigInt::from(3).pow(cur_ell - i);
            badprob += bin_coeff * term1 * term2;
        }

        badprob /= BigInt::from(4).pow(cur_ell);

        println!("ell = {}, badprob = {}", cur_ell, badprob);

        if badprob <= negl {
            println!("found ell = {}", cur_ell);
            ell = Some(cur_ell);
        }
    }

    ell
}

#[cfg(feature = "key_length_search")]
fn binomial_num(n: u32, k: u32) -> num_bigint::BigInt {
    use num_bigint::BigInt;
    use num_traits::One;

    let mut res = BigInt::one();

    for i in 1..=k {
        res *= n - i + 1;
        res /= i;
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::garble::InterstellarCircuit;

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
        let mut d_up: HashMap<&WireRef, (BlockL, BlockL)> = HashMap::new();
        let l0 = random_oracle.new_random_blockL();
        let l1 = random_oracle.new_random_blockL();
        d_up.insert(&circuit_outputs[0], (l0.clone(), l1.clone()));

        let d = decoding_info(&circuit_outputs, d_up, &mut random_oracle);
        let dj = &d.d[0];
        assert_eq!(random_oracle.random_oracle_prime(&l0, dj), false);
        assert_eq!(random_oracle.random_oracle_prime(&l1, dj), true);
    }

    #[cfg(feature = "key_length_search")]
    #[test]
    fn test_key_length_search() {
        assert_eq!(key_length_search_num(1700, 1800).unwrap(), 42);
    }
}
