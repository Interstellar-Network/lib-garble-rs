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

struct Circuit {}

struct Block {
    val: u128,
}

impl Block {
    fn random() -> Self {
        // TODO proper random; or better use Scuttlebutt directly
        Block { val: 42 }
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub(crate) struct Wire {
    val: bool,
}

/// "Collectively, the set of labels associated with the wire is denoted by {Kj}"
struct K_labels_set {
    value0: Block,
    value1: Block,
}

/// "The Label Sampling Function f0 This function assigns an l-bit label Kj to
/// each possible value that wire j can take. Collectively, the set of labels associated
/// with the wire is denoted by {Kj }. In particular, Yao’s scheme and all subsequent
/// optimizations decompose the circuit’s input into bits and each bit is assigned a
/// label (See also [App17]).""
fn f0_label_sampling(wire: Wire) -> K_labels_set {
    K_labels_set {
        value0: Block::random(),
        value1: Block::random(),
    }
}

struct CompressedSet {
    x00: Block,
    x01: Block,
    x10: Block,
    x11: Block,
}

// TODO should probably be deterministic? or random?
// use some kind of hash?
fn random_oracle(label_a: &Block, label_b: &Block) -> Block {
    todo!()
}

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
fn f1_0_compress(wire_a: K_labels_set, wire_b: K_labels_set) -> CompressedSet {
    CompressedSet {
        x00: random_oracle(&wire_a.value0, &wire_b.value0),
        x01: random_oracle(&wire_a.value0, &wire_b.value1),
        x10: random_oracle(&wire_a.value1, &wire_b.value0),
        x11: random_oracle(&wire_a.value1, &wire_b.value1),
    }
}

struct Delta {}

// "Collapse.
// These four outputs of the random oracle are given to f1,1 to produce
// ∇ (this is either ∇⊕ or ∇∧, depending on the gate type)"
fn f1_1_collapse(compressed_set: CompressedSet) -> Delta {
    todo!()
}
