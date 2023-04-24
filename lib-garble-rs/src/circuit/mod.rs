mod gate;
mod skcd_config;

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
    // TODO?
    // pub(crate) inputs: Vec<GateRef>,
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

    /// Evaluate (clear text version == UNGARBLED) using "mcircuit"
    /// For simplicity, this only supports "evaluator_inputs" b/c this is only
    /// used to test basic circuits(eg adders, etc) so no point in having 2PC.
    pub(crate) fn eval_plain(&self, evaluator_inputs: &[u16]) -> Vec<u16> {
        use mcircuit::evaluate_composite_program;
        use mcircuit::{CombineOperation, Operation, WireValue};

        assert!(
            self.num_evaluator_inputs() == self.circuit.n(),
            "only basic circuits wihout garbler inputs! [1]"
        );
        assert!(
            self.num_garbler_inputs() == 0,
            "only basic circuits wihout garbler inputs! [2]"
        );

        let mut circuit = vec![];

        for evaluator_input in evaluator_inputs {
            circuit.push(CombineOperation::GF2(Operation::Input(
                evaluator_input.clone() as usize,
            )));
        }

        // TODO do we want "self" to be mutable or not?
        let mut skcd_to_wire_ref_converter = self.circuit.skcd_to_wire_ref_converter.clone();

        // cf https://github.com/trailofbits/mcircuit/blob/8fe9b315f2e8cae6020a2884ae544d59bd0bbd41/src/parsers/blif.rs#L194
        // For how to match blif/skcd gates into mcircuit's Operation
        for gate in &self.circuit.gates {
            match &gate.internal {
                GateInternal::Standard {
                    r#type,
                    input_a,
                    input_b,
                } => match r#type {
                    // GateType::AANB => todo!(),
                    // GateType::INVB => todo!(),
                    // GateType::NAAB => todo!(),
                    GateType::INV => circuit.push(CombineOperation::GF2(Operation::AddConst(
                        input_a.clone().unwrap().id,
                        0,
                        true,
                    ))),
                    GateType::XOR => circuit.push(CombineOperation::GF2(Operation::Add(
                        input_a.clone().unwrap().id,
                        input_b.clone().unwrap().id,
                        gate.output.id,
                    ))),
                    GateType::NAND => {
                        // NAND is a AND, whose output is NOTed
                        let nand_intermediate_output_id = format!("NAND_temp_{}", gate.output.id);
                        skcd_to_wire_ref_converter.insert(&nand_intermediate_output_id);
                        let nand_intermediate_output = skcd_to_wire_ref_converter
                            .get(&nand_intermediate_output_id)
                            .unwrap();

                        let op_and = CombineOperation::GF2(Operation::Mul(
                            input_a.clone().unwrap().id,
                            input_b.clone().unwrap().id,
                            nand_intermediate_output.id,
                        ));
                        let op_not = CombineOperation::GF2(Operation::AddConst(
                            nand_intermediate_output.id,
                            gate.output.id,
                            true,
                        ));
                        circuit.push(op_and);
                        circuit.push(op_not);
                    }
                    GateType::AND => circuit.push(CombineOperation::GF2(Operation::Mul(
                        input_a.clone().unwrap().id,
                        input_b.clone().unwrap().id,
                        gate.output.id,
                    ))),
                    // GateType::XNOR => todo!(),
                    GateType::BUF => circuit.push(CombineOperation::GF2(Operation::AddConst(
                        input_a.clone().unwrap().id,
                        0,
                        false,
                    ))),
                    // GateType::AONB => todo!(),
                    // GateType::BUFB => todo!(),
                    // GateType::NAOB => todo!(),
                    // GateType::OR => todo!(),
                    // GateType::NOR => todo!(),
                    // GateType::ONE => todo!(),
                    // GateType::ZERO => todo!(),
                    _ => todo!("unsupported gate type! [{:?}]", gate),
                },
                GateInternal::Constant { value } => circuit.push(CombineOperation::GF2(
                    Operation::Const(value.clone() as usize, true),
                )),
            }
        }

        let bool_inputs: Vec<bool> = evaluator_inputs
            .iter()
            .map(|input| input.clone() == 0)
            .collect();
        evaluate_composite_program(&circuit, &bool_inputs, &[]);
        todo!()
    }
}
