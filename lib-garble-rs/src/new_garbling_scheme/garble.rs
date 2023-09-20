use alloc::vec::Vec;
use bytes::BytesMut;
use hashbrown::{HashMap, HashSet};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;

use circuit_types_rs::{Circuit, Gate, GateType, KindUnary, WireRef};

use super::{
    block::BlockL, circuit_for_eval::CircuitForEval, delta, random_oracle::RandomOracle,
    wire::Wire, wire_labels_set::WireLabelsSet,
};

#[derive(Debug, Snafu)]
pub(crate) enum GarblerError {
    /// During `fn garble`, when looping on the Gates in order,
    /// they SHOULD be processed in topological order.
    /// ie if a Gate is used as input for other Gates, it SHOULD be processed before them!
    GateIdOutputMismatch,
    EvaluateDuplicatedWire,
    /// "Algorithm 5 Gate" L15/16
    /// "15: if HW (∇g )̸ = ℓ then 16: ABORT the computation"
    BadHammingWeight {
        hw: usize,
    },
    /// error during `garble_internal`: the wire is NOT present in the "current wires set"
    GarbleMissingWire {
        wire: WireRef,
    },
    /// error during `decoding_info`: the wire is NOT present in `D`
    DecodedInfoMissingWire {
        output_wire: WireRef,
    },
    /// When calling `deltas.try_insert` the key was already present;
    /// It SHOULD NOT happen b/c we are processing gate by gate!
    DeltaAlreadyPresent {
        delta_key: WireRef,
    },
    /// Error at `BlockP::get_bit` the given index is not valid wrt the internal `self.bits_words`/`get_bits_internal`
    BlockPBitOutOfRange {
        index: usize,
    },
}

/// In <https://eprint.iacr.org/2021/739.pdf>
/// this is the lines 1 to 4 of "Algorithm 5 Gate"
/// 1: Xg00 = `ROg` (LA0 , LB0 )
/// 2: Xg01 = `ROg` (LA0 , LB1 )
/// 3: Xg10 = `ROg` (LA1 , LB0 )
/// 4: Xg11 = `ROg` (LA1 , LB1 )
///
/// Also called `Compress` in <https://www.esat.kuleuven.be/cosic/publications/article-3351.pdf>
/// The function f1,0, which we model as a random oracle, is used to
/// compress each pair into a random string of length Lprime, i.e.,
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
fn f1_0_compress(
    encoded_wires: &[Option<Wire>],
    gate: &Gate,
    input_a: &WireRef,
    input_b: &WireRef,
    buf: &mut BytesMut,
) -> Result<WireLabelsSet, GarblerError> {
    let tweak = gate.get_id();

    let wire_a: &Wire =
        encoded_wires[input_a.id]
            .as_ref()
            .ok_or_else(|| GarblerError::GarbleMissingWire {
                wire: input_a.clone(),
            })?;
    let wire_b: &Wire =
        encoded_wires[input_b.id]
            .as_ref()
            .ok_or_else(|| GarblerError::GarbleMissingWire {
                wire: input_b.clone(),
            })?;

    Ok(WireLabelsSet::new_binary(
        RandomOracle::random_oracle_g(wire_a.value0(), Some(wire_b.value0()), tweak, buf),
        RandomOracle::random_oracle_g(wire_a.value0(), Some(wire_b.value1()), tweak, buf),
        RandomOracle::random_oracle_g(wire_a.value1(), Some(wire_b.value0()), tweak, buf),
        RandomOracle::random_oracle_g(wire_a.value1(), Some(wire_b.value1()), tweak, buf),
    ))
}

/// "input encoding set e."
///
/// NOTE: Contrary to the papers it is a `HashMap` instead of a Vec in topological order
/// b/c in `fn garble` when looping on `circuit.gates` the gate.id is NOT guaranteed to be in order!
/// eg
/// - circuits inputs: *should* indeed usually be in order => for instance 0..2
/// - BUT the first "Gate ID" could be eg 5
/// - which means the second iteration of the loop would not work without a hashmap
///
/// Produced by: `garble::init_internal`
/// Used by: `garble::garble_internal`, `evaluate::encoding_internal`
///
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub(super) struct InputEncodingSet {
    /// One per input
    pub(super) e: Vec<Wire>,
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
/// See "Algorithm 4 Circuit" in <https://eprint.iacr.org/2021/739.pdf>
/// up to 5:
///
/// See also:
/// "Algorithm 3 Init(C, ℓ)" in <https://www.esat.kuleuven.be/cosic/publications/article-3351.pdf>
///
/// 1: extract n from C and initialize e = []
/// 2:  for input wire W ∈ [n] do
/// 3:      Sample LW0 ← {0, 1}ℓ uniformly at random
/// 4:      Sample LW1 ← {0, 1}ℓ − {LW0 } uniformly at random
/// 5:      Set e[W ] = eW = (LW0 , LW1 )
/// 6:  end for
/// 7: Return e
///
/// param `r`: [Supporting Free-XOR] this is the "delta" for Free-XOR; ie a random `BlockL`
///
fn init_internal(circuit: &Circuit, rng: &mut ChaChaRng, r: &BlockL) -> InputEncodingSet {
    let nb_inputs = circuit.get_nb_inputs();
    let mut w = Vec::with_capacity(nb_inputs);
    for (idx, input_wire) in circuit.get_wires()[0..nb_inputs].iter().enumerate() {
        // CHECK: the Wires MUST be iterated in topological order!
        assert_eq!(
            input_wire.id, idx,
            "Wires MUST be iterated in topological order!"
        );

        insert_new_wire_random_labels(rng, &mut w, r);
    }

    // w.extend((0..circuit.q()).iter(). )

    // assert_eq!(w.len(), circuit.n() as usize + circuit.q(), "wrong w length! [3]");

    // w

    InputEncodingSet { e: w }
}

/// Generate a new RANDOM wire
/// [Supporting Free-XOR]
/// - l0 is random
/// - l1 is based on XOR l0 and `r`
///   "invariant that for the output wire of the XOR gate, L0 ⊕ L1 = ∆"
///   5 Supporting Free-XOR; <https://eprint.iacr.org/2021/739.pdf>
///
/// param: r: [Supporting Free-XOR] "delta"
fn insert_new_wire_random_labels(rng: &mut ChaChaRng, wires: &mut Vec<Wire>, _r: &BlockL) {
    let lw0 = RandomOracle::new_random_block_l(rng);
    let lw1 = RandomOracle::new_random_block_l(rng);

    // NOTE: if this fails: add a diff(cf pseudocode) or xor or something like that
    assert!(lw0 != lw1, "LW0 and LW1 MUST NOT be the same!");
    // [Supporting Free-XOR]
    // assert_eq!(&lw0.xor(&lw1), r, "LW0 and LW1 SHOULD match `r` XOR!");

    wires.push(Wire::new(lw0, lw1));
}

/// Garble
///
/// In <https://eprint.iacr.org/2021/739.pdf>
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
fn garble_internal(
    circuit: &Circuit,
    e: &InputEncodingSet,
) -> Result<GarbledCircuitInternal, GarblerError> {
    // "6: initialize F = [], D = []"
    let mut f = Vec::new();
    // "+ 1" b/c get_max_gate_id is a valid ID to be processed!
    f.resize_with(
        circuit.get_metadata().get_max_gate_id() + 1,
        Default::default,
    );
    // also noted as: ∇g
    // TODO should this (semantically) be instead `HashMap<&WireRef, Wire>`(or `HashMap<&WireRef, &Wire>`)
    let mut deltas = HashMap::with_capacity(circuit.get_nb_outputs());

    // As we are looping on the gates in order, this will be built step by step
    // ie the first gates are inputs, and this will already contain them.
    // Then we built all the other gates in subsequent iterations of the loop.
    let mut encoded_wires: Vec<Option<Wire>> = Vec::new();
    encoded_wires.resize_with(circuit.get_nb_wires(), Default::default);
    for (idx, input_wire) in e.e.iter().enumerate() {
        encoded_wires[idx] = Some(input_wire.clone());
    }

    // [constant gate special case]
    // We need a placeholder Wire for simplicity; these are NOT used during `evaluate_internal` etc
    let constant_block0 = BlockL::new_with([0, 0]);
    let constant_block1 = BlockL::new_with([u64::MAX, u64::MAX]);

    // DEBUG `InputEncodingSet`
    // let all_wires: Vec<usize> = Vec::from_iter(e.e.keys().map(|w| w.id));
    // let mut all_wires_sorted = all_wires.clone();
    // all_wires_sorted.sort();

    let outputs_set: HashSet<&WireRef> = circuit.get_outputs().iter().collect();
    let mut buf = BytesMut::new();

    for gate in circuit.get_gates().iter().flatten() {
        let (l0, l1): (BlockL, BlockL) = match gate.get_type() {
            // STANDARD CASE: Binary Gates or using Delta etc
            GateType::Binary {
                gate_type,
                input_a,
                input_b,
            } => {
                let compressed_set =
                    f1_0_compress(&encoded_wires, gate, input_a, input_b, &mut buf)?;
                let (l0, l1, delta) = delta::Delta::new(&compressed_set, gate_type)?;
                f[gate.get_id()] = Some(delta);
                (l0.into(), l1.into())
            }
            // SPECIAL CASE: Unary Gates are bypassing Delta (and therefore DO NOT need a RO call during eval)
            GateType::Unary { gate_type, input_a } => {
                let wire_a: &Wire = encoded_wires[input_a.id].as_ref().ok_or_else(|| {
                    GarblerError::GarbleMissingWire {
                        wire: input_a.clone(),
                    }
                })?;

                match gate_type {
                    // https://www.cs.toronto.edu/~vlad/papers/XOR_ICALP08.pdf
                    // "We first note that NOT gates can be implemented “for free”
                    // by simply eliminating them and inverting the correspondence of the wires’ values
                    // and garblings."
                    KindUnary::INV => (wire_a.value1().clone(), wire_a.value0().clone()),
                    // We apply the same idea to BUF Gates: a simple "passthrough"
                    KindUnary::BUF => (wire_a.value0().clone(), wire_a.value1().clone()),
                }
            }
            // [constant gate special case]
            GateType::Constant { value: _ } => (constant_block0.clone(), constant_block1.clone()),
        };

        // TODO what index should we use?
        // w is init with [0,n], and as size [0,n+q]
        // what about Gate's index? (== output)
        let new_wires = Wire::new(l0, l1);
        encoded_wires[gate.get_id()] = Some(new_wires.clone());

        // "12: if g is an output gate then"
        // TODO(opt) if circuit_metadata.gate_idx_is_output(wire_ref.id) { (cf `evaluate_internal`)
        if let Some(wire_output) = outputs_set.get(gate.get_output()) {
            deltas
                .try_insert(
                    (*wire_output).clone().clone(),
                    (new_wires.value0().clone(), new_wires.value1().clone()),
                )
                .map_err(|_| GarblerError::DeltaAlreadyPresent {
                    delta_key: (*wire_output).clone(),
                })?;
        }
    }

    // assert_eq!(encoded_wires, deltas);
    Ok(GarbledCircuitInternal {
        f: F { f },
        d: D { d: deltas },
    })
}

/// Noted `F` in the paper
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub(super) struct F {
    /// One per Gate, or rather per [free-XOR] non-free Gate
    /// But for ease of implementation we use Option<> and f.len() == "nb of gates"
    pub(super) f: Vec<Option<delta::Delta>>,
}

/// Noted `D` in the paper
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct D {
    d: HashMap<WireRef, (BlockL, BlockL)>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub(super) struct GarbledCircuitInternal {
    pub(super) f: F,
    d: D,
}

/// This is the EVALUABLE `GarbledCircuit`; ie the result of the whole garbling pipeline.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub(crate) struct GarbledCircuitFinal {
    pub(crate) circuit: CircuitForEval,
    pub(super) garbled_circuit: GarbledCircuitInternal,
    pub(super) d: DecodedInfo,
    pub(super) e: InputEncodingSet,
    pub(crate) eval_metadata: EvalMetadata,
}

/// Similar to `CircuitMetadata` but only what is needed during evaluation(instead of during garbling)
#[derive(PartialEq, Debug, Deserialize, Serialize, Clone)]
pub(crate) struct EvalMetadata {
    pub(crate) nb_outputs: usize,
}

/// Grouping of all of the sequence:
/// (1) Init(C) → e;
/// (2) Circuit(C, e) = (F, D);
/// (3) DecodingInfo(D) → d
///
/// # Arguments
///
/// * `rng_seed` - when None; will use the standard and secure `ChaChaRng::from_entropy`
///     when given: wil use the NOT SECURE `seed_from_u64`
///
// TODO? how to group the garble part vs eval vs decoding?
pub(crate) fn garble(
    circuit: Circuit,
    rng_seed: Option<u64>,
) -> Result<GarbledCircuitFinal, GarblerError> {
    let mut rng = if let Some(rng_seed) = rng_seed {
        ChaChaRng::seed_from_u64(rng_seed)
    } else {
        ChaChaRng::from_entropy()
    };

    // [Supporting Free-XOR] this is the "delta" for Free-XOR; ie a random BlockL
    let r = RandomOracle::new_random_block_l(&mut rng);

    let e = init_internal(&circuit, &mut rng, &r);

    let garbled_circuit = garble_internal(&circuit, &e)?;

    let d = decoding_info(circuit.get_outputs(), &garbled_circuit.d, &mut rng)?;

    let eval_metadata = EvalMetadata {
        nb_outputs: circuit.get_outputs().len(),
    };

    Ok(GarbledCircuitFinal {
        circuit: circuit.into(),
        garbled_circuit,
        d,
        e,
        eval_metadata,
    })
}

/// Noted `d` in the paper
///
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub(super) struct DecodedInfo {
    /// One element per output
    pub(super) d: Vec<BlockL>,
}

/// In <https://eprint.iacr.org/2021/739.pdf>
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
    rng: &mut ChaChaRng,
) -> Result<DecodedInfo, GarblerError> {
    let mut d = Vec::with_capacity(circuit_outputs.len());
    let mut buf = BytesMut::new();

    // "2: for output wire j ∈ [m] do"
    for (_idx, output_wire) in circuit_outputs.iter().enumerate() {
        // "extract Lj0, Lj1 ← D[j]"
        let (lj0, lj1) =
            d_up.d
                .get(output_wire)
                .ok_or_else(|| GarblerError::DecodedInfoMissingWire {
                    output_wire: output_wire.clone(),
                })?;

        let mut dj = RandomOracle::new_random_block_l(rng);
        loop {
            let a = !RandomOracle::random_oracle_prime(lj0, &dj, &mut buf);
            let b = RandomOracle::random_oracle_prime(lj1, &dj, &mut buf);
            if a && b {
                break;
            }
            dj = RandomOracle::new_random_block_l(rng);
        }

        d.push(dj);
    }

    Ok(DecodedInfo { d })
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use rand::SeedableRng;
    use rand_chacha::ChaChaRng;

    use super::*;

    #[test]
    fn test_decoding_info() {
        let circuit_outputs = vec![WireRef { id: 42 }];
        let mut d_up = HashMap::new();
        let mut rng = ChaChaRng::from_entropy();
        let l0 = RandomOracle::new_random_block_l(&mut rng);
        let l1 = RandomOracle::new_random_block_l(&mut rng);
        d_up.insert(circuit_outputs[0].clone(), (l0.clone(), l1.clone()));

        let d = D { d: d_up };

        let d = decoding_info(&circuit_outputs, &d, &mut rng).unwrap();
        let dj = &d.d[0];
        let mut buf = BytesMut::new();
        assert!(!RandomOracle::random_oracle_prime(&l0, dj, &mut buf));
        assert!(RandomOracle::random_oracle_prime(&l1, dj, &mut buf));
    }
}
