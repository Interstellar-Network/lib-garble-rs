use hashbrown::{hash_map::OccupiedError, HashMap, HashSet};

use crate::{
    circuit::{Circuit, Gate, GateType, WireRef},
    garble::GarblerError,
};

use super::{
    block::BlockL, delta, random_oracle::RandomOracle, wire::Wire, wire_labels_set::WireLabelsSet,
};

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
fn f1_0_compress(w: &InputEncodingSet, gate: &Gate) -> WireLabelsSet {
    let tweak = gate.get_id();

    match gate.get_type() {
        GateType::Binary {
            r#type,
            input_a,
            input_b,
        } => {
            let wire_a: &Wire = &w.e[input_a];
            let wire_b: &Wire = &w.e[input_b];

            WireLabelsSet::new_binary(
                RandomOracle::random_oracle_g(&wire_a.value0(), Some(&wire_b.value0()), tweak),
                RandomOracle::random_oracle_g(&wire_a.value0(), Some(&wire_b.value1()), tweak),
                RandomOracle::random_oracle_g(&wire_a.value1(), Some(&wire_b.value0()), tweak),
                RandomOracle::random_oracle_g(&wire_a.value1(), Some(&wire_b.value1()), tweak),
            )
        }
        GateType::Unary { r#type, input_a } => {
            let wire_a: &Wire = &w.e[input_a];

            WireLabelsSet::new_unary(
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
pub(super) struct InputEncodingSet {
    pub(super) e: HashMap<WireRef, Wire>,
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

/// Noted `F` in the paper
pub(super) struct F {
    /// One per Gate
    pub(super) f: HashMap<WireRef, delta::Delta>,
}

/// Noted `D` in the paper
struct D {
    d: HashMap<WireRef, (BlockL, BlockL)>,
}

pub(super) struct GarbledCircuitInternal {
    pub(super) f: F,
    d: D,
}

/// This is the EVALUABLE GarbledCircuit; ie the result of the whole garbling pipeline.
pub(crate) struct GarbledCircuitFinal {
    pub(super) circuit: Circuit,
    pub(super) garbled_circuit: GarbledCircuitInternal,
    pub(super) d: DecodedInfo,
    pub(super) e: InputEncodingSet,
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

/// Noted `d` in the paper
///
pub(super) struct DecodedInfo {
    pub(super) d: HashMap<WireRef, BlockL>,
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

#[cfg(test)]
mod tests {
    use super::*;

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
