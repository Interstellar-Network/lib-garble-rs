use crate::circuit::InterstellarCircuit;
use crate::circuit::SkcdConfig;
use fancy_garbling::classic::{garble, Encoder, GarbledCircuit};
use fancy_garbling::errors::EvaluatorError;

pub use fancy_garbling::classic::EvalCache;

#[cfg(all(not(feature = "std"), feature = "sgx"))]
use sgx_tstd::vec::Vec;

pub type EvaluatorInput = u16;

// TODO(interstellar) this is NOT good?? It requires the "non garbled" Circuit to be kept around
// we SHOULD (probably) rewrite "pub fn eval" in fancy-garbling/src/circuit.rs to to NOT use "self",
// and replace "circuit" by a list of ~~Gates~~/Wires?? [cf how "cache" is constructed in "fn eval"]
pub struct InterstellarGarbledCircuit {
    pub(crate) garbled: GarbledCircuit,
    pub(crate) encoder: Encoder,
    pub config: SkcdConfig,
}

#[derive(Debug)]
pub enum InterstellarEvaluatorError {
    FancyError(EvaluatorError),
}

impl InterstellarGarbledCircuit {
    pub(crate) fn garble(circuit: InterstellarCircuit) -> Self {
        let (encoder, garbled) = garble(circuit.circuit).unwrap();
        InterstellarGarbledCircuit {
            garbled,
            encoder,
            config: circuit.config,
        }
    }

    pub fn eval(
        &mut self,
        garbler_inputs: &[EvaluatorInput],
        evaluator_inputs: &[EvaluatorInput],
    ) -> Result<Vec<u16>, InterstellarEvaluatorError> {
        let evaluator_inputs = self.encoder.encode_evaluator_inputs(evaluator_inputs);
        let garbler_inputs = self.encoder.encode_garbler_inputs(garbler_inputs);

        self.garbled
            .eval(&garbler_inputs, &evaluator_inputs)
            .map_err(InterstellarEvaluatorError::FancyError)
    }

    pub fn eval_with_prealloc(
        &mut self,
        garbler_inputs: &[EvaluatorInput],
        evaluator_inputs: &[EvaluatorInput],
        outputs: &mut Vec<Option<u16>>,
        eval_cache: &mut EvalCache,
    ) -> Result<(), InterstellarEvaluatorError> {
        let evaluator_inputs = self.encoder.encode_evaluator_inputs(evaluator_inputs);
        let garbler_inputs = self.encoder.encode_garbler_inputs(garbler_inputs);

        self.garbled
            .eval_with_prealloc(eval_cache, &garbler_inputs, &evaluator_inputs, outputs)
            .map_err(InterstellarEvaluatorError::FancyError)?;

        Ok(())
    }

    pub fn init_cache(&mut self) -> EvalCache {
        self.garbled.init_cache()
    }
}
