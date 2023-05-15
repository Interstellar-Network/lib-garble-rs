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

use serde::{Deserialize, Serialize};

use crate::circuit::Circuit;
use crate::circuit::{Gate, GateInternal, GateType, WireRef};

mod block;
mod constant;
mod delta;
mod random_oracle;

use block::{BlockL, BlockP};
use delta::DeltaTable;
use random_oracle::RandomOracle;

use self::delta::Delta;

type WireInternal = bool;

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub(crate) struct Wire {
    val: WireInternal,
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

struct CompressedSet {
    x00: BlockP,
    x01: BlockP,
    x10: BlockP,
    x11: BlockP,
}

impl CompressedSet {
    /// In https://eprint.iacr.org/2021/739.pdf this is a helper for
    /// "Algorithm 5 Gate"
    /// 7: Set slice ← Xg00[j]||Xg01[j]||Xg10[j]||Xg11[j]
    ///
    /// Return the specific BIT for each x00,x01,x10,x11
    pub(super) fn get_bits_slice(&self, index: usize) -> (bool, bool, bool, bool) {
        return (
            self.x00.get_bit(index),
            self.x01.get_bit(index),
            self.x10.get_bit(index),
            self.x11.get_bit(index),
        );
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
fn f1_0_compress(wire_a: &K_label, wire_b: &K_label, gate: &Gate) -> CompressedSet {
    let tweak = gate.output.id;
    CompressedSet {
        x00: RandomOracle::random_oracle_g(&wire_a.value0, &wire_b.value0, tweak),
        x01: RandomOracle::random_oracle_g(&wire_a.value0, &wire_b.value1, tweak),
        x10: RandomOracle::random_oracle_g(&wire_a.value1, &wire_b.value0, tweak),
        x11: RandomOracle::random_oracle_g(&wire_a.value1, &wire_b.value1, tweak),
    }
}

/// Compute the ∇ = f1.1 in the paper
/// "Collapse.
/// These four outputs of the random oracle are given to f1,1 to produce
/// ∇ (this is either ∇⊕ or ∇∧, depending on the gate type)"
fn f1_1_collapse(compressed_set: CompressedSet, gate: &Gate) -> Delta {
    let mut delta_table = DeltaTable::new_for_gate(gate);

    // TODO is this ALWAYS project_x00_delta or should it depend on gate type?
    // TODO how to generalize s1 formula for any gate type?
    // let s0 = f11_res.project_x00_delta();
    // let s1 = f11_res.compute_s1();

    Delta::new_from_delta_table(delta_table, &compressed_set)
}

/// "Algorithm 3 Init(C, ℓ)"
///
/// 1: extract n from C and initialize e = []
/// 2:  for input wire W ∈ [n] do
/// 3:      Sample LW0 ← {0, 1}ℓ uniformly at random
/// 4:      Sample LW1 ← {0, 1}ℓ − {LW0 } uniformly at random
/// 5:      Set e[W ] = eW = (LW0 , LW1 )
/// 6:  end for
/// 7: Return e
fn init(circuit: &Circuit, random_oracle: &mut RandomOracle) -> Vec<K_label> {
    let mut e = vec![];
    for input_wire in &circuit.wires()[0..circuit.n() as usize] {
        let lw0 = random_oracle.new_random_block();
        let lw1 = random_oracle.new_random_block();

        // NOTE: if this fails: add a diff(cf pseudocode) or xor or something like that
        assert!(lw0 != lw1, "LW0 and LW1 MUST NOT be the same!");

        e.push(K_label {
            value0: lw0,
            value1: lw1,
        });
    }

    assert_eq!(e.len(), circuit.inputs.len(), "wrong e length! [1]");
    assert_eq!(e.len(), circuit.n() as usize, "wrong e length! [2]");

    e
}

pub(crate) fn garble(circuit: Circuit) {
    // "External length parameter"
    let l = constant::KAPPA;
    // "Internal length parameter"
    let l_prime = 8 * l;

    let mut random_oracle = RandomOracle::new();

    let e = init(&circuit, &mut random_oracle);

    for gate in &circuit.gates {
        match &gate.internal {
            GateInternal::Standard {
                r#type,
                input_a,
                input_b,
            } => {
                // TODO how to handle unwrap() based on gate type?
                let wire_a = &e[input_a.as_ref().unwrap().id];
                let wire_b = &e[input_b.as_ref().unwrap().id];

                let compressed_set = f1_0_compress(wire_a, wire_b, gate);
                let f11_res = f1_1_collapse(compressed_set, gate);

                // let k0 = RandomOracle::random_oracle_1(&s0);
                // let k1 = RandomOracle::random_oracle_1(&s1);

                match r#type {
                    // GateType::INV => todo!(),
                    GateType::XOR => todo!(),
                    // GateType::NAND => todo!(),
                    GateType::AND => todo!(),
                    // ite = If-Then-Else
                    // we define BUF as "if input == 1 then input; else 0"
                    // GateType::BUF => todo!(),
                    _ => todo!("unsupported gate type! [{:?}]", gate),
                }
            } // TODO?
              // GateInternal::Constant { value } => todo!(),
        };
    }
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

    #[cfg(feature = "key_length_search")]
    #[test]
    fn test_key_length_search() {
        assert_eq!(key_length_search_num(1700, 1800).unwrap(), 42);
    }
}
