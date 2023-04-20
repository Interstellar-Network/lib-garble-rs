mod gate;
mod skcd_config;

pub(crate) use gate::{Gate, GateInternal, GateRef, GateType};
pub(crate) use skcd_config::{
    DisplayConfig, EvaluatorInputs, EvaluatorInputsType, GarblerInputs, GarblerInputsType,
    SkcdConfig,
};

/// "Circuit syntax. A Boolean circuit C : {0, 1}n → {0, 1}m has n input wires
/// enumerated by the indices 1, . . . , n, and m output wires enumerated by n + q −
/// m + 1, . . . , n + q, where q = |C| is the number Boolean gates. The output wire
/// of gate j (also denoted by gj ) is n + j,"
pub(crate) struct Circuit {
    pub(crate) n: usize,
    pub(crate) m: usize,
    pub(crate) gates: Vec<gate::Gate>,
}

/// Represents the raw(ie **UN**garbled) circuit; usually created from a .skcd file
///
/// Exists mostly to mask swanky/fancy-garbling Circuit to the public.
pub(crate) struct InterstellarCircuit {
    pub(crate) circuit: Circuit,
    pub(crate) config: skcd_config::SkcdConfig,
}

impl InterstellarCircuit {
    pub(crate) fn num_evaluator_inputs(&self) -> usize {
        todo!()
    }

    pub(crate) fn eval_plain(&self, garbler_inputs: &[u16], evaluator_inputs: &[u16]) -> Vec<u16> {
        todo!()
    }
}
