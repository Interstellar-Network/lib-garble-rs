use fancy_garbling::circuit::Circuit;
use fancy_garbling::errors::DummyError;

/// Represents the raw(ie UNgarbled) circuit; usually created from a .skcd file
///
/// Exists mostly to mask swanky/fancy-garbling Circuit to the public.
pub struct InterstellarCircuit {
    pub(crate) circuit: Circuit,
}

/// Forward to the corresponding swanky/fancy-garbling functions
impl InterstellarCircuit {
    pub fn eval_plain(
        &self,
        garbler_inputs: &[u16],
        evaluator_inputs: &[u16],
    ) -> Result<Vec<u16>, DummyError> {
        self.circuit.eval_plain(garbler_inputs, evaluator_inputs)
    }

    pub fn num_evaluator_inputs(&self) -> usize {
        self.circuit.num_evaluator_inputs()
    }
}
