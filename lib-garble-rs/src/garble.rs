use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use circuit_types_rs::DisplayConfig;

use crate::new_garbling_scheme::evaluate::EncodedInfo;
use crate::new_garbling_scheme::garble::GarbledCircuitFinal;
use crate::new_garbling_scheme::wire_value::WireValue;
use crate::new_garbling_scheme::{self};
use crate::InterstellarEvaluatorError;
use crate::{EvalCache, InterstellarError};

pub type EvaluatorInput = u8;
pub(super) type GarblerInput = u8;
// TODO? proper struct to avoid implicit conversion?
// pub struct EvaluatorInput(u8);
// pub(super) struct GarblerInput(u8);

/// The main garbling part in mod `new_garbling_scheme` only handles "raw" circuits.
/// But using `SkcdConfig` we have added the concept of `GarblerInputs`(for the watermark/otp)
/// vs `EvaluatorInputs`(ie the random inputs during each render loop).
/// This struct is here to bridge the gap.
#[derive(PartialEq, Debug, Deserialize, Serialize, Clone)]
pub struct GarbledCircuit {
    // TODO DO NOT Serialize the full `GarbleCircuit`[at least not entirely]
    // MUST NOT be sent to the client-side b/c that probably leaks data
    // Instead we should just send the list of labels pair (0,1) for each EvaluatorInput only
    pub(super) garbled: GarbledCircuitFinal,
}

/// The logic of the inputs handling MUST be consistant (cf `num_evaluator_inputs`,`num_inputs` AND `eval`)
/// Here we decide to express "for generic circuit -> all inputs are evaluator inputs".
/// For example, a full adder will have only "evaluator inputs"(3) and 0 garbler inputs.
/// which means
/// - `num_inputs` MUST return 0 for a "generic circuit"
/// - `num_inputs` MUST return `garbler_inputs` for a "display circuit"
/// - `num_evaluator_inputs` MUST return `nb_inputs` for a "generic circuit"
/// - etc
/// We do it this way b/c it allows the callers to use the same eval logic for "generic" vs "display".
///
///
impl GarbledCircuit {
    pub(super) fn new(garbled: GarbledCircuitFinal) -> Self {
        Self { garbled }
    }

    /// [INTERNAL]
    /// This is used as a sort of `fn is_display_circuit() -> bool` if a circuit is a "generic" or a "display" one
    /// This is used by the `pub` functions treating the inputs eg `num_inputs`,`encode_garbler_inputs`,etc
    ///
    fn get_config_internal(&self) -> &Option<DisplayConfig> {
        self.garbled.circuit.get_config()
    }

    #[must_use]
    pub fn num_evaluator_inputs(&self) -> usize {
        match self.get_config_internal() {
            Some(config) => config.num_evaluator_inputs() as usize,
            None => self.garbled.circuit.get_nb_inputs(),
        }
    }

    #[must_use]
    pub fn num_inputs(&self) -> usize {
        match self.get_config_internal() {
            Some(config) => config.num_garbler_inputs() as usize,
            None => 0,
        }
    }

    /// ONLY for "generic circuits"
    /// for "display circuits" use the corresponding `num_evaluator_inputs` and `num_inputs`
    #[must_use]
    pub fn num_outputs(&self) -> usize {
        self.garbled.eval_metadata.nb_outputs
    }

    /// Return the `display_config`, originally cloned from the original `Circuit`
    ///
    /// # Errors
    /// - `NotAValidDisplayCircuit`: DO NOT call on a "generic circuit", ONLY use on "display circuits"!
    ///
    pub fn get_display_config(&self) -> Result<&DisplayConfig, InterstellarError> {
        self.get_config_internal()
            .as_ref()
            .ok_or(InterstellarError::NotAValidDisplayCircuit)
    }

    /// (Sort of) ONLY for "display circuits"
    /// For "generic circuits", you SHOULD only use `fn eval`, and skip the call to `encode_inputs` entirely
    /// cf struct docstring for details.
    /// For "generic circuits" this is a simple noop; needed b/c we still need the output for serialization.
    ///
    pub(super) fn encode_inputs(&self, inputs: &[GarblerInput]) -> EncodedGarblerInputs {
        if self.get_config_internal().is_some() {
            self.encode_garbler_inputs_internal(inputs)
        } else {
            self.encode_garbler_inputs_internal(&[])
        }
    }

    /// ONLY for "display circuits"
    /// for "generic circuits" use the corresponding `encode_inputs`
    pub(super) fn encode_garbler_inputs_internal(
        &self,
        garbler_inputs: &[GarblerInput],
    ) -> EncodedGarblerInputs {
        // TODO(interstellar)? but is this the correct time to CHECK?
        let expected_inputs_len = self.num_inputs();
        assert_eq!(
            expected_inputs_len,
            garbler_inputs.len(),
            "wrong garbler_inputs len!"
        );

        // convert param `garbler_inputs` into `WireValue`
        let garbler_inputs_wire_value: Vec<WireValue> = garbler_inputs
            .iter()
            .map(core::convert::Into::into)
            .collect();

        EncodedGarblerInputs {
            encoded: new_garbling_scheme::evaluate::encode_garbler_inputs(
                &self.garbled,
                &garbler_inputs_wire_value,
                0,
                expected_inputs_len,
            ),
        }
    }

    /// Evaluate
    /// This is meant to be called repeatedly in the render loop so it is trying
    /// to `in-place` as much as possible.
    ///
    /// # Errors
    ///
    /// `FancyError` if something went wrong during **either** eval(now)
    /// or initially when garbling!
    /// In the latter case it means the circuit is a dud and nothing can be done!
    pub fn eval(
        &self,
        encoded_garbler_inputs: &EncodedGarblerInputs,
        evaluator_inputs: &[EvaluatorInput],
        outputs: &mut Vec<u8>,
        eval_cache: &mut EvalCache,
    ) -> Result<(), InterstellarEvaluatorError> {
        // convert param `garbler_inputs` into `WireValue`
        let evaluator_inputs_wire_value: Vec<WireValue> = evaluator_inputs
            .iter()
            .map(core::convert::Into::into)
            .collect();

        // TODO(opt) remove clone
        let mut encoded_info = encoded_garbler_inputs.encoded.clone();

        new_garbling_scheme::evaluate::encode_evaluator_inputs(
            &self.garbled,
            &evaluator_inputs_wire_value,
            &mut encoded_info,
            self.num_inputs(),
            self.num_inputs() + self.num_evaluator_inputs(),
        );

        // TODO this SHOULD have `outputs` in-place [1]
        let outputs_wire_value = new_garbling_scheme::evaluate::evaluate_with_encoded_info(
            &self.garbled,
            &encoded_info,
            eval_cache,
        )?;

        // Convert Vec<WireValue> -> Vec<u8>
        let outputs_u8: Vec<u8> = outputs_wire_value
            .into_iter()
            .map(core::convert::Into::into)
            .collect();
        *outputs = outputs_u8;

        Ok(())
    }
}

/// `EncodedGarblerInputs`: sent to the client as part of `EvaluableGarbledCircuit`
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct EncodedGarblerInputs {
    pub(super) encoded: EncodedInfo,
}
