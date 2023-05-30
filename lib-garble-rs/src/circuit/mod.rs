mod gate;
mod skcd_config;

use serde::{Deserialize, Serialize};

pub(crate) use gate::{Gate, GateType, GateTypeBinary, GateTypeUnary, WireRef};
pub(crate) use skcd_config::{
    DisplayConfig, EvaluatorInputs, EvaluatorInputsType, GarblerInputs, GarblerInputsType,
    SkcdConfig, SkcdToWireRefConverter,
};

/// "Circuit syntax. A Boolean circuit C : {0, 1}n → {0, 1}m has n input wires
/// enumerated by the indices 1, . . . , n, and m output wires enumerated by n + q −
/// m + 1, . . . , n + q, where q = |C| is the number Boolean gates. The output wire
/// of gate j (also denoted by gj ) is n + j,"
///
/// NOTE: this is important, especially for the outputs to be in order!
/// ie DO NOT use HashSet/HashMap etc in this struct!
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub(crate) struct CircuitInternal {
    pub(crate) num_garbler_inputs: u32,
    pub(crate) num_evaluator_inputs: u32,
    pub(crate) inputs: Vec<WireRef>,
    pub(crate) outputs: Vec<WireRef>,
    pub(crate) gates: Vec<gate::Gate>,
    pub(crate) wires: Vec<WireRef>,
    pub(crate) wire_constant0: WireRef,
    pub(crate) wire_constant1: WireRef,
}

impl CircuitInternal {
    /// Return "n" ie the number of inputs
    pub(crate) fn n(&self) -> u32 {
        self.num_garbler_inputs + self.num_evaluator_inputs
    }

    /// Return "m" ie the number of wires
    pub(crate) fn m(&self) -> u32 {
        self.wires.len().try_into().unwrap()
    }

    /// Return "q" ie the number of gates
    pub(crate) fn q(&self) -> u32 {
        self.gates.len().try_into().unwrap()
    }

    /// Return the list of all the wires.
    /// Used during garbling "init" function to create the "encoding".
    pub(crate) fn wires(&self) -> &Vec<WireRef> {
        &self.wires
    }
}

/// Represents the raw(ie **UN**garbled) circuit; usually created from a .skcd file
///
/// Exists mostly to mask swanky/fancy-garbling Circuit to the public.
pub(crate) struct Circuit {
    pub(crate) circuit: CircuitInternal,
    pub(crate) config: skcd_config::SkcdConfig,
}

#[cfg(test)]
#[derive(Debug, snafu::Snafu)]
pub enum EvaluateError {
    Unknown,
}

#[cfg(test)]
impl Circuit {
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
    ///
    /// NOTE: "expected_outputs" are passed as param b/c of the way "evaluate" from the crate "boolean_expression" works
    /// See also: https://stackoverflow.com/questions/59109453/how-do-i-use-the-rust-crate-boolean-expression-to-implement-a-simple-logic-cir
    pub(crate) fn eval_plain(&self, evaluator_inputs: &[u8]) -> Result<Vec<u8>, EvaluateError> {
        use boolean_expression::*;

        assert!(
            self.num_evaluator_inputs() == self.circuit.n(),
            "only basic circuits wihout garbler inputs! [1]"
        );
        assert!(
            self.num_garbler_inputs() == 0,
            "only basic circuits wihout garbler inputs! [2]"
        );

        let mut circuit = boolean_expression::BDD::new();
        // Map: "WireRef" == Gate ID to a BDDFunc
        let mut bdd_map = hashbrown::HashMap::new();

        for input_wire in &self.circuit.inputs {
            bdd_map.insert(input_wire.id, circuit.terminal(input_wire.id));
        }

        // cf https://github.com/trailofbits/mcircuit/blob/8fe9b315f2e8cae6020a2884ae544d59bd0bbd41/src/parsers/blif.rs#L194
        // For how to match blif/skcd gates into mcircuit's Operation
        // WARNING: apparently Operation::XXX is (OUTPUT, INPUT1, etc)! OUTPUT IS FIRST!
        for gate in &self.circuit.gates {
            let bdd_gate: BDDFunc = match gate.get_type() {
                GateType::Binary {
                    gate_type: r#type,
                    input_a,
                    input_b,
                } => match r#type {
                    GateTypeBinary::XOR => circuit.xor(
                        bdd_map
                            .get(&input_a.id)
                            .expect("GateType::XOR missing input a!")
                            .clone(),
                        bdd_map
                            .get(&input_b.id)
                            .expect("GateType::XOR missing input b!")
                            .clone(),
                    ),
                    GateTypeBinary::XNOR => {
                        // XNOR is a XOR, whose output is NOTed
                        let xor_output = circuit.xor(
                            bdd_map
                                .get(&input_a.id)
                                .expect("GateType::XOR missing input a!")
                                .clone(),
                            bdd_map
                                .get(&input_b.id)
                                .expect("GateType::XOR missing input b!")
                                .clone(),
                        );

                        circuit.not(xor_output)
                    }
                    GateTypeBinary::NAND => {
                        // NAND is a AND, whose output is NOTed
                        let and_output = circuit.and(
                            bdd_map
                                .get(&input_a.id)
                                .expect("GateType::NAND missing input a!")
                                .clone(),
                            bdd_map
                                .get(&input_b.id)
                                .expect("GateType::NAND missing input b!")
                                .clone(),
                        );

                        circuit.not(and_output)
                    }
                    GateTypeBinary::NOR => {
                        // NOR is a OR, whose output is NOTed
                        let or_output = circuit.or(
                            bdd_map
                                .get(&input_a.id)
                                .expect("GateType::NOR missing input a!")
                                .clone(),
                            bdd_map
                                .get(&input_b.id)
                                .expect("GateType::NOR missing input b!")
                                .clone(),
                        );

                        circuit.not(or_output)
                    }
                    GateTypeBinary::AND => circuit.and(
                        bdd_map
                            .get(&input_a.id)
                            .expect("GateType::AND missing input a!")
                            .clone(),
                        bdd_map
                            .get(&input_b.id)
                            .expect("GateType::AND missing input b!")
                            .clone(),
                    ),
                    GateTypeBinary::OR => circuit.or(
                        bdd_map
                            .get(&input_a.id)
                            .expect("GateType::OR missing input a!")
                            .clone(),
                        bdd_map
                            .get(&input_b.id)
                            .expect("GateType::OR missing input b!")
                            .clone(),
                    ),
                },
                GateType::Unary {
                    gate_type: r#type,
                    input_a,
                } => match r#type {
                    GateTypeUnary::INV => circuit.not(
                        bdd_map
                            .get(&input_a.id)
                            .expect("GateType::NOT missing input a!")
                            .clone(),
                    ),
                    // ite = If-Then-Else
                    // we define BUF as "if input == 1 then input; else 0"
                    GateTypeUnary::BUF => circuit.ite(
                        bdd_map
                            .get(&input_a.id)
                            .expect("GateType::NOT missing input a!")
                            .clone(),
                        BDD_ONE,
                        BDD_ZERO,
                    ),
                },
                GateType::Constant { value } => circuit.constant(value.clone()),
            };

            bdd_map.insert(gate.get_output().id, bdd_gate);
        }

        ////////////////////////////////////////////////////////////////////////

        let circuit = circuit.clone();

        // cf boolean_expression examples/tests for how the evaluation works
        // https://github.com/cfallin/boolean_expression/blob/795b89567e05f54907b89453bdd481d0b01f0c93/src/bdd.rs#L1071
        let hashmap_inputs = evaluator_inputs
            .iter()
            .enumerate()
            .map(|(idx, input)| (idx, input.clone() == 1))
            .collect();

        let res_outputs: Vec<u8> = self
            .circuit
            .outputs
            .iter()
            .map(|output| {
                let output_bddfunc = bdd_map.get(&output.id).expect("missing output!").clone();
                circuit.evaluate(output_bddfunc, &hashmap_inputs) as u8
            })
            .collect();
        println!("########### evaluate : {:?}", res_outputs);

        Ok(res_outputs)
    }

    /// Build a basic circuit containing only a desired Binary Gate
    pub(crate) fn new_test_circuit(gate_binary_type: GateTypeBinary) -> Self {
        Self {
            circuit: CircuitInternal {
                num_garbler_inputs: 2,
                num_evaluator_inputs: 0,
                inputs: vec![WireRef { id: 0 }, WireRef { id: 1 }],
                outputs: vec![WireRef { id: 2 }],
                gates: vec![Gate {
                    internal: GateType::Binary {
                        gate_type: gate_binary_type,
                        input_a: WireRef { id: 0 },
                        input_b: WireRef { id: 1 },
                    },
                    output: WireRef { id: 2 },
                }],
                wires: vec![WireRef { id: 0 }, WireRef { id: 1 }, WireRef { id: 2 }],
                wire_constant0: WireRef { id: 42 },
                wire_constant1: WireRef { id: 43 },
            },
            config: SkcdConfig {
                display_config: None,
                garbler_inputs: vec![],
                evaluator_inputs: vec![],
            },
        }
    }

    pub(crate) fn new_test_circuit_unary(gate_unary_type: GateTypeUnary) -> Self {
        Self {
            circuit: CircuitInternal {
                num_garbler_inputs: 1,
                num_evaluator_inputs: 0,
                inputs: vec![WireRef { id: 0 }],
                outputs: vec![WireRef { id: 1 }],
                gates: vec![Gate {
                    internal: GateType::Unary {
                        gate_type: gate_unary_type,
                        input_a: WireRef { id: 0 },
                    },
                    output: WireRef { id: 1 },
                }],
                wires: vec![WireRef { id: 0 }, WireRef { id: 1 }],
                wire_constant0: WireRef { id: 42 },
                wire_constant1: WireRef { id: 43 },
            },
            config: SkcdConfig {
                display_config: None,
                garbler_inputs: vec![],
                evaluator_inputs: vec![],
            },
        }
    }
}
