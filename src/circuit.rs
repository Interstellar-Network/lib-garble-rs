use fancy_garbling::circuit::Circuit;
use fancy_garbling::errors::DummyError;

// TODO!!! add the rest of skcd.proto
#[derive(Debug, Clone, Copy)]
pub struct DisplayConfig {
    pub width: u32,
    pub height: u32,
    // cf drawable::DigitSegmentsType
    // TODO!!! NOT PUB segments_type: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct SkcdConfig {
    pub display_config: Option<DisplayConfig>,
}

/// Represents the raw(ie UNgarbled) circuit; usually created from a .skcd file
///
/// Exists mostly to mask swanky/fancy-garbling Circuit to the public.
pub struct InterstellarCircuit {
    pub(crate) circuit: Circuit,
    pub(crate) config: SkcdConfig,
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
