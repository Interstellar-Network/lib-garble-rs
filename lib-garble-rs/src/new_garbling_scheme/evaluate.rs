use hashbrown::{hash_map::OccupiedError, HashMap, HashSet};
use serde::{Deserialize, Serialize};

use crate::{
    circuit::{CircuitInternal, GateType, WireRef},
    new_garbling_scheme::{wire::WireLabel, GarblerError},
};

use super::{
    block::{BlockL, BlockP},
    garble::{DecodedInfo, GarbledCircuitFinal, InputEncodingSet, F},
    random_oracle::RandomOracle,
    wire_value::WireValue,
};

/// Noted `X`
///
/// For each Circuit.inputs this will be a `Block` referencing either `value0` or `value1`
///
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub(crate) struct EncodedInfo {
    x: HashMap<WireRef, WireLabel>,
}

impl EncodedInfo {
    pub(crate) fn len(&self) -> usize {
        self.x.len()
    }
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
    circuit: &'a CircuitInternal,
    e: &'a InputEncodingSet,
    x: &'a [WireValue],
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
fn evaluate_internal(circuit: &CircuitInternal, f: &F, encoded_info: &EncodedInfo) -> OutputLabels {
    let mut output_labels = OutputLabels {
        y: HashMap::with_capacity(circuit.outputs.len()),
    };

    let outputs_set: HashSet<&WireRef> = HashSet::from_iter(circuit.outputs.iter());

    // "for each gate g ∈ [q] in a topological order do"
    for gate in circuit.gates.iter() {
        // "LA, LB ← active labels associated with the input wires of gate g"
        let (l_a, l_b) = match gate.get_type() {
            GateType::Binary {
                gate_type: r#type,
                input_a,
                input_b,
            } => {
                let l_a = encoded_info.x.get(input_a).unwrap();
                let l_b = encoded_info.x.get(input_b).unwrap();

                (l_a.get_block(), Some(l_b.get_block()))
            }
            GateType::Unary {
                gate_type: r#type,
                input_a,
            } => {
                let l_a = encoded_info.x.get(input_a).unwrap();
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
    circuit: &CircuitInternal,
    output_labels: &OutputLabels,
    decoded_info: &DecodedInfo,
) -> Vec<WireValue> {
    let mut outputs = vec![];

    // "for j ∈ [m] do"
    for output in circuit.outputs.iter() {
        // "y[j] ← lsb(RO′(Y [j], dj ))"
        let yj = output_labels.y.get(output).unwrap();
        let dj = decoded_info.d.get(output).unwrap();
        let r = RandomOracle::random_oracle_prime(yj, dj);
        // NOTE: `random_oracle_prime` directly get the LSB so no need to do it here
        outputs.push(WireValue { value: r });
    }

    outputs
}

/// Full evaluate chain
///
/// NOTE: this is mostly for testing purposes
/// The "standard" API is to do "multi step" eval with Garbler Inputs vs Evaluator Inputs
/// cf `encode_inputs` etc
///
pub(crate) fn evaluate(garbled: &GarbledCircuitFinal, x: &[WireValue]) -> Vec<WireValue> {
    let encoded_info = encoding_internal(&garbled.circuit, &garbled.e, x);

    let output_labels =
        evaluate_internal(&garbled.circuit, &garbled.garbled_circuit.f, &encoded_info);

    decoding_internal(&garbled.circuit, &output_labels, &garbled.d)
}

/// encoded inputs
/// ie convert a "vec" of bool/u8 into a "vec" of Wire Labels
pub(crate) fn encode_inputs(garbled: &GarbledCircuitFinal, x: &[WireValue]) -> EncodedInfo {
    encoding_internal(&garbled.circuit, &garbled.e, x)
}
