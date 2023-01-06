use crate::circuit::InterstellarCircuit;
use crate::circuit::SkcdConfig;
use fancy_garbling::classic::{garble, Encoder, GarbledCircuit};
use fancy_garbling::errors::EvaluatorError;
use fancy_garbling::Wire;
use serde::{Deserialize, Serialize};

pub use fancy_garbling::classic::EvalCache;

#[cfg(all(not(feature = "std"), feature = "sgx"))]
use sgx_tstd::vec::Vec;

pub type EvaluatorInput = u16;
type GarblerInput = u16;

// TODO(interstellar) this is NOT good?? It requires the "non garbled" Circuit to be kept around
// we SHOULD (probably) rewrite "pub fn eval" in fancy-garbling/src/circuit.rs to to NOT use "self",
// and replace "circuit" by a list of ~~Gates~~/Wires?? [cf how "cache" is constructed in "fn eval"]
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct InterstellarGarbledCircuit {
    pub(crate) garbled: GarbledCircuit,
    pub(crate) encoder: Encoder,
    pub config: SkcdConfig,
}

/// Obtained by calling Inter::
pub struct EncodedGarblerInputs {
    pub(crate) wires: Vec<Wire>,
}

#[derive(Debug)]
pub enum InterstellarEvaluatorError {
    FancyError(EvaluatorError),
}

impl InterstellarGarbledCircuit {
    /// NOTE: it is NOT pub b/c we want to only expose the full parse_skcd+garble, cf lib.rs
    pub(crate) fn garble(circuit: InterstellarCircuit) -> Self {
        let (encoder, garbled) = garble(circuit.circuit).unwrap();
        InterstellarGarbledCircuit {
            garbled,
            encoder,
            config: circuit.config,
        }
    }

    // TODO(interstellar) SHOULD NOT expose Wire; instead return a wrapper struct eg "GarblerInputs"
    pub fn encode_garbler_inputs(&self, garbler_inputs: &[GarblerInput]) -> EncodedGarblerInputs {
        EncodedGarblerInputs {
            wires: self.encoder.encode_garbler_inputs(garbler_inputs),
        }
    }

    // TODO(interstellar) #[cfg(test)]
    pub fn eval(
        &mut self,
        encoded_garbler_inputs: &EncodedGarblerInputs,
        evaluator_inputs: &[EvaluatorInput],
    ) -> Result<Vec<u16>, InterstellarEvaluatorError> {
        let evaluator_inputs = self.encoder.encode_evaluator_inputs(evaluator_inputs);

        self.garbled
            .eval(&encoded_garbler_inputs.wires, &evaluator_inputs)
            .map_err(InterstellarEvaluatorError::FancyError)
    }

    pub fn eval_with_prealloc(
        &mut self,
        encoded_garbler_inputs: &EncodedGarblerInputs,
        evaluator_inputs: &[EvaluatorInput],
        outputs: &mut Vec<Option<u16>>,
        eval_cache: &mut EvalCache,
    ) -> Result<(), InterstellarEvaluatorError> {
        let encoded_evaluator_inputs = self.encoder.encode_evaluator_inputs(evaluator_inputs);

        self.garbled
            .eval_with_prealloc(
                eval_cache,
                &encoded_garbler_inputs.wires,
                &encoded_evaluator_inputs,
                outputs,
            )
            .map_err(InterstellarEvaluatorError::FancyError)?;

        Ok(())
    }

    pub fn init_cache(&mut self) -> EvalCache {
        self.garbled.init_cache()
    }
}
