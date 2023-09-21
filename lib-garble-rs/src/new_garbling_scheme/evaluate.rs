use alloc::vec::Vec;
use bytes::BytesMut;
use serde::{Deserialize, Serialize};

use circuit_types_rs::WireRef;

use crate::{
    new_garbling_scheme::wire::WireLabel, EncodedGarblerInputs, GarbledCircuit,
    InterstellarEvaluatorError,
};

use super::{
    block::BlockL,
    circuit_for_eval::{CircuitForEval, GateTypeForEval},
    garble::{DecodedInfo, GarbledCircuitFinal, InputEncodingSet, F},
    random_oracle::RandomOracle,
    wire_value::WireValue,
};

#[cfg(feature = "std")]
use rayon::prelude::*;

/// Noted `X`
///
/// For each Circuit.inputs this will be a `Block` referencing either `value0` or `value1`
///
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub(crate) struct EncodedInfo {
    /// NOTE: contrary to the papers, we added the concept of "Garbler inputs" vs "Evaluator inputs"
    /// so we have an `Option<>` here:
    /// - "garbler inputs" are typically set at garbling time (ie server-side)
    /// - "evaluator inputs" are typically set later when evaluating (client-side); these are init to `None`
    ///   when garbling
    x: Vec<WireLabel>,
}

impl EncodedInfo {
    pub(crate) fn len(&self) -> usize {
        self.x.len()
    }
}

/// Encoding
///
/// NOTE: it is called both "server-side" when garling, and "client-side" when evaluating
/// Therefore `wire_start_index` and `wire_end_index`:
/// - "server-side" == "garbler inputs": SHOULD be `0..num_garbler_inputs`()
/// - "client-side" == "evaluator inputs": SHOULD be `num_garbler_inputs()+1..num_garbler_inputs()+1+num_evaluator_inputs`()
///
/// In <https://eprint.iacr.org/2021/739.pdf> "Algorithm 7"
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
/// In <https://www.esat.kuleuven.be/cosic/publications/article-3351.pdf>
/// Algorithm 4 Algorithm En(e, x)
///
/// 1: for every j ∈ [n] do
/// 2:  output Kjxj = ej [xj ]
/// 3: end for
fn encoding_internal<'a>(
    circuit: &'a CircuitForEval,
    e: &'a InputEncodingSet,
    inputs: &'a [WireValue],
    encoded_info: &mut EncodedInfo,
    inputs_start_index: usize,
    inputs_end_index: usize,
) {
    // CHECK: we SHOULD have one "user input" for each Circuit's input(ie == `circuit.n`)
    assert_eq!(
        inputs_end_index - inputs_start_index,
        inputs.len(),
        "encoding: `x` inputs len MUST match the Circuit's inputs len!"
    );

    // NOTE: contrary to the papers, we added the concept of "Garbler inputs" vs "Evaluator inputs"
    // which means the loop is in a different order.
    // ie we loop of the "wire value"(given by the user/evaluator/garbler) instead of the `circuit.inputs`
    for (input_wire, input_value) in circuit.get_inputs()[inputs_start_index..inputs_end_index]
        .iter()
        .zip(inputs)
    {
        let encoded_wire = &e.e[input_wire.id];
        let block = if input_value.value {
            encoded_wire.value1()
        } else {
            encoded_wire.value0()
        };
        encoded_info.x.push(WireLabel::new(block));
    }

    // InputEncodingSet: SHOULD contain circuit.n elements
    // encoded_info: SHOULD have a CAPACITY of circuit.n elements
    //               BUT at this point in time (eg for "garbler inputs") we MAY only have set the first circuit.num_garbler_inputs!
    // assert_eq!(
    //     encoded_info.x.len(),
    //     e.e.len(),
    //     "EncodedInfo: wrong length!"
    // );
}

/// Noted `Y` in the paper
#[derive(Clone)]
pub(super) struct OutputLabels {
    /// One element per output
    y: Vec<BlockL>,
}

impl OutputLabels {
    pub fn new() -> Self {
        Self { y: Vec::new() }
    }
}

/// This is what is needed to evaluate in-place as much as possible
/// ie a bunch of "temp vec" and various "buffers"
pub struct EvalCache {
    output_labels: OutputLabels,
    /// one per "output" (ie len() == circuit.outputs.len())
    /// This is used to avoid alloc in `decoding_internal` during eval
    outputs_bufs: Vec<BytesMut>,
    // For the main "gate loop": we read on one, and write on the other `Vec<WireLabel>`
    // for a given layer; and for the next one it switches
    wire_labels_tuple: (Vec<WireLabel>, Vec<WireLabel>),
    // That is is the reference init with eg `encoded_garbler_inputs.encoded.x`
    // it is cloned into `wire_labels_tuple` at the start of each layer processing
    // It avoid a loop; and SHOULD(hopefully) be compiled into a simple memcopy
    wire_labels_base: Vec<WireLabel>,
}

impl EvalCache {
    #[must_use]
    pub fn new(garbled: &GarbledCircuit, encoded_garbler_inputs: &EncodedGarblerInputs) -> Self {
        // TODO TOREMOVE wire_labels.resize_with(circuit.get_gates().len(), Default::default);
        // TODO move out! This is duplicating init code for each layer...
        // same idea as `garble`:
        // As we are looping on the gates in order, this will be built step by step
        // ie the first gates are inputs, and this will already contain them.
        // Then we built all the other gates in subsequent iterations of the loop.
        let mut wire_labels_base: Vec<WireLabel> = vec![];

        init_wire_labels_base(
            &mut wire_labels_base,
            &garbled.garbled.circuit,
            &encoded_garbler_inputs.encoded,
        );

        Self {
            output_labels: OutputLabels::new(),
            outputs_bufs: Vec::new(),
            wire_labels_tuple: (
                vec![WireLabel::default(); garbled.garbled.circuit.get_nb_wires()],
                vec![WireLabel::default(); garbled.garbled.circuit.get_nb_wires()],
            ),
            wire_labels_base,
        }
    }
}

/// Called twice AT MOST:
/// - once from `EvalCache::new`; but `encoded_garbler_inputs` CAN be empty so it may do nothing
/// - directly from `evaluate_internal` to "init if needed"
fn init_wire_labels_base(
    wire_labels_base: &mut Vec<WireLabel>,
    circuit: &CircuitForEval,
    encoded_info: &EncodedInfo,
) {
    wire_labels_base.resize_with(circuit.get_nb_wires(), Default::default);

    if wire_labels_base[0..encoded_info.x.len()] == encoded_info.x {
        return;
    }

    for (idx, wire_label) in encoded_info.x.iter().enumerate() {
        wire_labels_base[idx] = wire_label.clone();
    }

    assert_eq!(&wire_labels_base[0..encoded_info.x.len()], &encoded_info.x);
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
#[allow(clippy::unnecessary_lazy_evaluations)]
fn evaluate_internal(
    circuit: &CircuitForEval,
    f: &F,
    encoded_info: &EncodedInfo,
    output_labels: &mut OutputLabels,
    wire_labels_tuple: &mut (Vec<WireLabel>, Vec<WireLabel>),
    wire_labels_base: &mut Vec<WireLabel>,
) -> Result<(), InterstellarEvaluatorError> {
    // CHECK: we SHOULD have one "user input" for each Circuit's input(ie == `circuit.n`)
    assert_eq!(
        encoded_info.x.len(),
        circuit.get_nb_inputs(),
        "encoding: `encoded_info` inputs len MUST match the Circuit's inputs len!"
    );

    output_labels
        .y
        .resize_with(circuit.get_nb_outputs(), Default::default);

    // [constant gate special case]
    // we need a placeholder Wire for simplicity
    // TODO move into EvalCache
    let constant_block0 = BlockL::new_with([0, 0]);
    let constant_block1 = BlockL::new_with([u64::MAX, u64::MAX]);

    let mut ro_buf = BytesMut::new();
    let circuit_metadata = circuit.get_metadata();

    // TODO TOREMOVE
    // let mut wire_labels = &mut wire_labels_per_layer[0];
    let nb_layers = circuit.get_gates().len();

    // "for each gate g ∈ [q] in a topological order do"
    for (layer_idx, gates) in circuit.get_gates().iter().enumerate() {
        // // : (&mut Vec<WireLabel>, &mut Vec<WireLabel>)
        // let split_idx = (layer_idx + 1).min(nb_layers - 1);
        // let splitted = wire_labels_per_layer.split_at_mut(split_idx);
        // let (read_wire_labels, write_wire_labels) = (splitted.0.last().unwrap(), splitted.1);
        // let mut write_wire_labels = &mut wire_labels_per_layer[(layer_idx + 1).max(nb_layers - 1)];
        // let mut read_wire_labels = &wire_labels_per_layer[layer_idx];

        // TODO move out! This is duplicating init code for each layer...
        // same idea as `garble`:
        // As we are looping on the gates in order, this will be built step by step
        // ie the first gates are inputs, and this will already contain them.
        // Then we built all the other gates in subsequent iterations of the loop.

        // TODO WTF; cleanup? swap???
        wire_labels_tuple.1 = wire_labels_tuple.0.clone();
        wire_labels_tuple.0[0..encoded_info.x.len()].copy_from_slice(&encoded_info.x);
        // wire_labels_tuple.1[0..encoded_info.x.len()].copy_from_slice(&encoded_info.x);

        assert_eq!(
            &wire_labels_tuple.0[0..encoded_info.x.len()],
            &encoded_info.x
        );

        // NOTE: b/c rayon (rightfully) prevents us to write `wire_labels` in parallel
        // - first we parallelize gate_layer -> and produce a temp Vec<BlockL>
        // - then we copy this "temp Vec<BlockL>" into the parameter `wire_labels`
        //
        // START rayon
        // gates.par_iter().enumerate().try_for_each_with(
        //     BytesMut::new(),
        //     |ro_buf, (gate_idx, gate)| {
        // END rayon
        // START single thread
        gates.iter().enumerate().try_for_each(|(gate_idx, gate)| {
            // END single thread
            let wire_ref = WireRef { id: gate.get_id() };

            let l_g: BlockL = match gate.get_type() {
                // STANDARD CASE: cf `garble_internal`
                GateTypeForEval::Binary { input_a, input_b } => {
                    // "LA, LB ← active labels associated with the input wires of gate g"
                    let l_a = &wire_labels_tuple.0[input_a.id];
                    let l_b = &wire_labels_tuple.0[input_b.id];

                    // "extract ∇g ← F [g]"
                    let delta_g_blockl = f.f[wire_ref.id]
                        .as_ref()
                        .ok_or_else(|| InterstellarEvaluatorError::EvaluateErrorMissingDelta {
                            idx: wire_ref.id,
                        })?
                        .get_block();

                    // "compute Lg ← RO(g, LA, LB ) ◦ ∇g"
                    let r = RandomOracle::random_oracle_g_truncated(
                        l_a.get_block(),
                        Some(l_b.get_block()),
                        gate.get_id(),
                        &mut ro_buf,
                    );
                    let l_g: BlockL = BlockL::new_projection(&r, delta_g_blockl);

                    l_g
                }
                // SPECIAL CASE: cf `garble_internal`
                GateTypeForEval::Unary { input_a } => {
                    let l_a = &wire_labels_tuple.0[input_a.id];
                    l_a.get_block().clone()
                }
                // [constant gate special case]
                // The `GateType::Constant` gates DO NOT need a garled representation.
                // They are evaluated directly.
                // That is b/c knowing is it is a TRUE/FALSE gate already leaks all there is to leak, so no point
                // in garbling...
                GateTypeForEval::Constant { value } => match value {
                    false => constant_block0.clone(),
                    true => constant_block1.clone(),
                },
            };

            wire_labels_tuple.1[wire_ref.id] = WireLabel::new(&l_g);
            // TODO? IMPORTANT: we MUST write into ALL the "next layers"
            // for wire_labels in &mut *write_wire_labels {
            //     wire_labels[wire_ref.id] = WireLabel::new(&l_g);
            // }

            Ok::<(), InterstellarEvaluatorError>(())
        })?;

        core::mem::swap(&mut wire_labels_tuple.0, &mut wire_labels_tuple.1);

        // TODO TOREMOVE assert_eq!(temp_gate_layer_labels.len(), gates.len());

        // TODO wire_labels[wire_ref.id] = Some(WireLabel::new(&l_g));
        // for (wire_label, temp_wire_label) in
        //     wire_labels.iter_mut().zip(temp_gate_layer_labels.iter())
        // {
        //     *wire_label = temp_gate_layer_labels;
        // }
        // for (gate_idx, gate) in gates.iter().enumerate() {
        //     let wire_ref = WireRef { id: gate.get_id() };
        //     wire_labels[wire_ref.id] = WireLabel::new(
        //         &temp_gate_layer_labels[gate_idx]
        //             .as_ref()
        //             .expect("temp_gate_layer_labels missing idx"),
        //     );

        //     // "if g is a circuit output wire then"
        //     // TODO move the previous lines under the if; or better: iter only on output gates? (filter? or circuit.outputs?)
        //     if circuit_metadata.gate_idx_is_output(wire_ref.id) {
        //         // "Y [g] ← Lg"
        //         output_labels.y[circuit_metadata.convert_gate_id_to_outputs_index(wire_ref.id)] =
        //             Some(
        //                 temp_gate_layer_labels[gate_idx]
        //                     .as_ref()
        //                     .expect("temp_gate_layer_labels missing idx")
        //                     .clone(),
        //             );
        //     }
        // }
    }

    // "if g is a circuit output wire then"
    // So here we:
    // - WRITE into `output_labels.y` so between indexes [0..end]
    // - by READING from wire_labels's slice (circuit_metadata.outputs_start_end_indexes.0..circuit_metadata.outputs_start_end_indexes.1)
    //
    // for wire_ref_id in 0..circuit_metadata.get_max_gate_id() + 1 {
    //     if circuit_metadata.gate_idx_is_output(wire_ref_id) {
    assert_eq!(
        output_labels.y.len(),
        circuit_metadata.get_outputs_range().len()
    );
    // REFERENCE
    // for wire_ref_id in circuit_metadata.get_outputs_range() {
    //     // "Y [g] ← Lg"
    //     let output_label_idx = circuit_metadata.convert_gate_id_to_outputs_index(wire_ref_id);
    //     output_labels.y[output_label_idx] = Some(wire_labels[wire_ref_id].get_block().clone());
    // } // OK
    // TODO .last().unwrap()
    // let last_wire_labels = wire_labels_per_layer.last().unwrap();
    // core::mem::swap(&mut wire_labels_tuple.0, &mut wire_labels_tuple.1);
    // IMPORTANT: the swap is at the end of the "main gate loop"; so read from `.0`!
    let last_wire_labels = &wire_labels_tuple.0;
    output_labels
        .y
        .par_iter_mut()
        .enumerate()
        .for_each(|(output_label_idx, output_label)| {
            *output_label = last_wire_labels
                [output_label_idx + circuit_metadata.get_outputs_start_index()]
            .get_block()
            .clone();
        });

    Ok(())
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
#[allow(clippy::unnecessary_lazy_evaluations)]
fn decoding_internal(
    outputs_bufs: &mut Vec<BytesMut>,
    output_labels: &OutputLabels,
    decoded_info: &DecodedInfo,
) -> Result<Vec<WireValue>, InterstellarEvaluatorError> {
    // TODO(rayon) make it work in work in no_std
    // #[cfg(not(feature = "std"))]
    // for output in circuit.outputs.iter() {

    // "for j ∈ [m] do"
    #[cfg(feature = "std")]
    let outputs = outputs_bufs
        .par_iter_mut()
        .enumerate()
        .map(|(idx, output_buf)| {
            // "y[j] ← lsb(RO′(Y [j], dj ))"
            let yj: &BlockL = &output_labels.y[idx];
            let dj = &decoded_info.d[idx];
            let r = RandomOracle::random_oracle_prime(yj, dj, output_buf);
            // NOTE: `random_oracle_prime` directly get the LSB so no need to do it here
            Ok(WireValue { value: r })
        })
        .collect();

    #[cfg(not(feature = "std"))]
    let outputs = outputs_bufs
        .iter_mut()
        .enumerate()
        .map(|(idx, output_buf)| {
            // "y[j] ← lsb(RO′(Y [j], dj ))"
            let yj = output_labels.y[idx].as_ref().ok_or_else(|| {
                InterstellarEvaluatorError::DecodingErrorMissingOutputLabel { idx }
            })?;
            let dj = &decoded_info.d[idx];
            let r = RandomOracle::random_oracle_prime(yj, dj, output_buf);
            // NOTE: `random_oracle_prime` directly get the LSB so no need to do it here
            Ok(WireValue { value: r })
        })
        .collect();

    outputs
}

/// Full evaluate chain
///
/// NOTE: this is mostly for testing purposes
///
/// The "standard" API is to do "multi step" eval with Garbler Inputs vs Evaluator Inputs
/// cf `encode_inputs` etc
///
#[cfg(test)]
pub(crate) fn evaluate_full_chain(
    garbled: &GarbledCircuitFinal,
    inputs: &[WireValue],
) -> Result<Vec<WireValue>, InterstellarEvaluatorError> {
    let mut encoded_info = EncodedInfo {
        x: Vec::with_capacity(inputs.len()),
    };

    encoding_internal(
        &garbled.circuit,
        &garbled.e,
        inputs,
        &mut encoded_info,
        0,
        garbled.circuit.get_nb_inputs(),
    );

    let mut output_labels = OutputLabels { y: Vec::new() };
    // TODO(opt) pass from param? (NOT that critical b/c only used for tests)
    let mut ro_buf = BytesMut::new();
    let mut eval_cache = EvalCache::new(
        &GarbledCircuit {
            garbled: garbled.clone(),
        },
        &EncodedGarblerInputs {
            encoded: encoded_info.clone(),
        },
    );

    evaluate_internal(
        &garbled.circuit,
        &garbled.garbled_circuit.f,
        &encoded_info,
        &mut output_labels,
        &mut eval_cache.wire_labels_tuple,
        &mut eval_cache.wire_labels_base,
    )?;

    // TODO(opt) pass from param? (NOT that critical b/c only used for tests)
    let mut outputs_bufs = Vec::new();
    outputs_bufs.resize_with(garbled.eval_metadata.nb_outputs, BytesMut::new);

    decoding_internal(&mut outputs_bufs, &output_labels, &garbled.d)
}

/// "Standard" evaluate chain
///
/// NOTE: this is the variant used in PROD with "garbler inputs" vs "evaluator inputs"
///
/// The "standard" API is to do "multi step" eval with Garbler Inputs vs Evaluator Inputs
/// cf `encode_inputs` etc
///
// TODO this SHOULD have `outputs` in-place [2]
pub(crate) fn evaluate_with_encoded_info(
    garbled: &GarbledCircuitFinal,
    encoded_info: &EncodedInfo,
    eval_cache: &mut EvalCache,
) -> Result<Vec<WireValue>, InterstellarEvaluatorError> {
    evaluate_internal(
        &garbled.circuit,
        &garbled.garbled_circuit.f,
        encoded_info,
        &mut eval_cache.output_labels,
        &mut eval_cache.wire_labels_tuple,
        &mut eval_cache.wire_labels_base,
    )?;

    // The correct size MUST be set!
    // Else we end up with the wrong number of outputs
    eval_cache
        .outputs_bufs
        .resize_with(garbled.eval_metadata.nb_outputs, BytesMut::new);

    decoding_internal(
        &mut eval_cache.outputs_bufs,
        &eval_cache.output_labels,
        &garbled.d,
    )
}

/// encoded inputs
/// "server-side" == "garbler inputs"
///
/// ie convert a "vec" of bool/u8 into a "vec" of Wire Labels
pub(crate) fn encode_garbler_inputs(
    garbled: &GarbledCircuitFinal,
    inputs: &[WireValue],
    inputs_start_index: usize,
    inputs_end_index: usize,
) -> EncodedInfo {
    let mut encoded_info = EncodedInfo {
        x: Vec::with_capacity(garbled.circuit.get_nb_inputs()),
    };

    encoding_internal(
        &garbled.circuit,
        &garbled.e,
        inputs,
        &mut encoded_info,
        inputs_start_index,
        inputs_end_index,
    );

    encoded_info
}

/// encoded inputs
/// "client-side" == "evaluator inputs"
///
/// ie convert a "vec" of bool/u8 into a "vec" of Wire Labels
pub(crate) fn encode_evaluator_inputs(
    garbled: &GarbledCircuitFinal,
    inputs: &[WireValue],
    encoded_info: &mut EncodedInfo,
    inputs_start_index: usize,
    inputs_end_index: usize,
) {
    encoding_internal(
        &garbled.circuit,
        &garbled.e,
        inputs,
        encoded_info,
        inputs_start_index,
        inputs_end_index,
    );
}
