mod gate;
mod skcd_config;

use std::collections::HashMap;

pub(crate) use gate::{Gate, GateInternal, GateType, WireRef};
pub(crate) use skcd_config::{
    DisplayConfig, EvaluatorInputs, EvaluatorInputsType, GarblerInputs, GarblerInputsType,
    SkcdConfig, SkcdToWireRefConverter,
};

/// "Circuit syntax. A Boolean circuit C : {0, 1}n → {0, 1}m has n input wires
/// enumerated by the indices 1, . . . , n, and m output wires enumerated by n + q −
/// m + 1, . . . , n + q, where q = |C| is the number Boolean gates. The output wire
/// of gate j (also denoted by gj ) is n + j,"
pub(crate) struct Circuit {
    pub(crate) num_garbler_inputs: u32,
    pub(crate) num_evaluator_inputs: u32,
    pub(crate) inputs: Vec<WireRef>,
    pub(crate) m: u32,
    pub(crate) gates: Vec<gate::Gate>,
    #[cfg(test)]
    pub(crate) skcd_to_wire_ref_converter: skcd_config::SkcdToWireRefConverter,
}

impl Circuit {
    /// Return "n" ie the number of inputs
    pub(crate) fn n(&self) -> u32 {
        self.num_garbler_inputs + self.num_evaluator_inputs
    }
}

/// Represents the raw(ie **UN**garbled) circuit; usually created from a .skcd file
///
/// Exists mostly to mask swanky/fancy-garbling Circuit to the public.
pub(crate) struct InterstellarCircuit {
    pub(crate) circuit: Circuit,
    pub(crate) config: skcd_config::SkcdConfig,
}

#[cfg(test)]
impl InterstellarCircuit {
    pub(crate) fn num_evaluator_inputs(&self) -> u32 {
        let mut num_inputs = 0;
        for skcd_input in &self.config.evaluator_inputs {
            num_inputs += skcd_input.length;
        }

        num_inputs
    }

    fn num_garbler_inputs(&self) -> u32 {
        let mut num_inputs = 0;
        for skcd_input in &self.config.garbler_inputs {
            num_inputs += skcd_input.length;
        }

        num_inputs
    }

    /// Evaluate (clear text version == UNGARBLED) using crate "boolean_expression"
    /// For simplicity, this only supports "evaluator_inputs" b/c this is only
    /// used to test basic circuits(eg adders, etc) so no point in having 2PC.
    pub(crate) fn eval_plain(&self, evaluator_inputs: &[u16]) -> Vec<u16> {
        use boolean_expression::*;

        assert!(
            self.num_evaluator_inputs() == self.circuit.n(),
            "only basic circuits wihout garbler inputs! [1]"
        );
        assert!(
            self.num_garbler_inputs() == 0,
            "only basic circuits wihout garbler inputs! [2]"
        );

        let mut circuit = BDD::new();
        // Map: "WireRef" == Gate ID to a BDDFunc
        let mut bdd_map = HashMap::new();

        // TODO remove field Circuit.inputs?
        // for (idx, _evaluator_input) in evaluator_inputs.iter().enumerate() {
        //     circuit.push(CombineOperation::Z64(Operation::Input(idx)));
        // }
        for input_wire in &self.circuit.inputs {
            bdd_map.insert(input_wire.id, circuit.terminal(input_wire.id));
        }

        // cf https://github.com/trailofbits/mcircuit/blob/8fe9b315f2e8cae6020a2884ae544d59bd0bbd41/src/parsers/blif.rs#L194
        // For how to match blif/skcd gates into mcircuit's Operation
        // WARNING: apparently Operation::XXX is (OUTPUT, INPUT1, etc)! OUTPUT IS FIRST!
        for gate in &self.circuit.gates {
            let bdd_gate: BDDFunc = match &gate.internal {
                GateInternal::Standard {
                    r#type,
                    input_a,
                    input_b,
                } => match r#type {
                    GateType::INV => circuit.not(
                        bdd_map
                            .get(&input_a.as_ref().unwrap().id)
                            .expect("GateType::INV missing input!")
                            .clone(),
                    ),
                    GateType::XOR => circuit.xor(
                        bdd_map
                            .get(&input_a.as_ref().unwrap().id)
                            .expect("GateType::XOR missing input a!")
                            .clone(),
                        bdd_map
                            .get(&input_b.as_ref().unwrap().id)
                            .expect("GateType::XOR missing input b!")
                            .clone(),
                    ),
                    GateType::NAND => {
                        // NAND is a AND, whose output is NOTed
                        let and_output = circuit.and(
                            bdd_map
                                .get(&input_a.as_ref().unwrap().id)
                                .expect("GateType::NAND missing input a!")
                                .clone(),
                            bdd_map
                                .get(&input_b.as_ref().unwrap().id)
                                .expect("GateType::NAND missing input b!")
                                .clone(),
                        );

                        circuit.not(and_output)
                    }
                    GateType::AND => circuit.and(
                        bdd_map
                            .get(&input_a.as_ref().unwrap().id)
                            .expect("GateType::AND missing input a!")
                            .clone(),
                        bdd_map
                            .get(&input_b.as_ref().unwrap().id)
                            .expect("GateType::AND missing input b!")
                            .clone(),
                    ),
                    // ite = If-Then-Else
                    // we define BUF as "if input == 1 then input; else 0"
                    GateType::BUF => circuit.ite(
                        bdd_map
                            .get(&input_a.as_ref().unwrap().id)
                            .expect("GateType::BUF missing input a!")
                            .clone(),
                        bdd_map
                            .get(&input_a.as_ref().unwrap().id)
                            .expect("GateType::BUF missing input a!")
                            .clone(),
                        BDD_ZERO,
                    ),
                    _ => todo!("unsupported gate type! [{:?}]", gate),
                },
                GateInternal::Constant { value } => circuit.constant(value.clone()),
            };

            bdd_map.insert(gate.output.id, bdd_gate);
        }

        let bool_inputs: Vec<bool> = evaluator_inputs
            .iter()
            .map(|input| input.clone() == 1)
            .collect();
        let arith_inputs: Vec<u64> = evaluator_inputs
            .iter()
            .map(|input| input.clone() as u64)
            .collect();

        // circuit.sat(f)

        todo!()
    }
}
