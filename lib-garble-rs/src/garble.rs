use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::circuit::SkcdConfig;
use crate::new_garbling_scheme::evaluate::EncodedInfo;
use crate::new_garbling_scheme::garble::GarbledCircuitFinal;
use crate::InterstellarEvaluatorError;

use crate::new_garbling_scheme::wire_value::WireValue;
use crate::new_garbling_scheme::{self};
use crate::EvalCache;

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
    // Instead we should just send the list of pair (0,1) for each EvaluatorInput only
    pub(super) garbled: GarbledCircuitFinal,
    pub config: SkcdConfig,
}

impl GarbledCircuit {
    #[must_use]
    pub fn num_garbler_inputs(&self) -> u32 {
        self.config.num_garbler_inputs()
    }

    #[must_use]
    pub fn num_evaluator_inputs(&self) -> u32 {
        self.config.num_evaluator_inputs()
    }

    #[must_use]
    pub fn num_outputs(&self) -> usize {
        self.garbled.eval_metadata.nb_outputs
    }

    pub(super) fn encode_garbler_inputs(
        &self,
        garbler_inputs: &[GarblerInput],
    ) -> EncodedGarblerInputs {
        // TODO(interstellar)? but is this the correct time to CHECK?
        assert_eq!(
            self.num_garbler_inputs() as usize,
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
                self.num_garbler_inputs() as usize,
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
        outputs: &mut [u8],
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
            self.num_garbler_inputs() as usize,
            self.num_garbler_inputs() as usize + self.num_evaluator_inputs() as usize,
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
        outputs.copy_from_slice(&outputs_u8);

        Ok(())
    }
}

/// `EncodedGarblerInputs`: sent to the client as part of `EvaluableGarbledCircuit`
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct EncodedGarblerInputs {
    pub(super) encoded: EncodedInfo,
}
