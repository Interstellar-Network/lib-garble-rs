use crate::circuit::InterstellarCircuit;
use fancy_garbling::classic::{garble, Encoder, GarbledCircuit};
use fancy_garbling::errors::EvaluatorError;

pub struct InterstellarGarbledCircuit {
    garbled: GarbledCircuit,
    encoder: Encoder,
    circuit: InterstellarCircuit,
}

#[derive(Debug)]
pub enum InterstellarEvaluatorError {
    FancyError(EvaluatorError),
}

impl InterstellarGarbledCircuit {
    pub fn garble(circuit: InterstellarCircuit) -> Self {
        let (encoder, garbled) = garble(&circuit.circuit).unwrap();
        InterstellarGarbledCircuit {
            garbled: garbled,
            encoder: encoder,
            circuit: circuit,
        }
    }

    pub fn eval(
        &self,
        evaluator_inputs: &[u16],
        garbler_inputs: &[u16],
    ) -> Result<Vec<u16>, InterstellarEvaluatorError> {
        let evaluator_inputs = &self.encoder.encode_evaluator_inputs(&evaluator_inputs);
        let garbler_inputs = &self.encoder.encode_garbler_inputs(&garbler_inputs);

        self.garbled
            .eval(&self.circuit.circuit, garbler_inputs, evaluator_inputs)
            .map_err(|e| InterstellarEvaluatorError::FancyError(e))
    }
}
