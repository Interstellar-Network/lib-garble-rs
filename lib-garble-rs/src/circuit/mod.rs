mod gate;
mod skcd_config;

use hashbrown::HashMap;
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
    pub(crate) inputs: Vec<WireRef>,
    pub(crate) outputs: Vec<WireRef>,
    pub(crate) gates: Vec<gate::Gate>,
    pub(crate) wires: Vec<WireRef>,
}

impl CircuitInternal {
    /// Return "n" ie the number of inputs
    pub(crate) fn n(&self) -> usize {
        self.inputs.len()
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
    pub(super) metadata: CircuitMetadata,
}

/// Various stats, min/max indexes for other fields, etc
/// This is useful both for debugging/info, and for eg optimising alloc
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub(super) struct CircuitMetadata {
    /// When garbling and evaluating, we iterate on the `gates` in topological order
    /// and we need to know if a gate is an output to perform special treatment on it.
    /// So instead of having to look-up inside `outputs`(or a temp corresponding hashset), we directly
    /// store the indices.
    /// NOTE: these are essentially INDICES refering to `gates`
    outputs_start_end_indexes: (usize, usize),
    // BELOW fields are mostly for Debug/Stats/etc
    gates_unary_count: HashMap<GateTypeUnary, usize>,
    gates_binary_count: HashMap<GateTypeBinary, usize>,
    /// The max gate.get_id() we can find in circuit.gates(typically get_id() == gate.output)
    /// We need this to init the proper vec to store "deltas"
    max_gate_id: usize,
}

impl CircuitMetadata {
    pub(super) fn new(outputs_start_end_indexes: (usize, usize), max_gate_id: usize) -> Self {
        Self {
            outputs_start_end_indexes,
            gates_unary_count: HashMap::new(),
            gates_binary_count: HashMap::new(),
            max_gate_id,
        }
    }

    /// param `idx`: SHOULD be an index from `CircuitInternal.gates`
    pub(super) fn gate_idx_is_output(&self, idx: usize) -> bool {
        (idx >= self.outputs_start_end_indexes.0) && (idx <= self.outputs_start_end_indexes.1)
    }

    pub(super) fn increment_unary_gate(&mut self, gate_type: &GateTypeUnary) {
        self.gates_unary_count
            .entry(gate_type.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    pub(super) fn increment_binary_gate(&mut self, gate_type: &GateTypeBinary) {
        self.gates_binary_count
            .entry(gate_type.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    /// When building `output_labels`, we need to insert elements base on
    /// the current gate ID; but in `decoding_internal` we directly loop on the outputs, NOT on the gates.
    /// So we need to map eg "gate ID" vs "output index"
    pub(crate) fn convert_gate_id_to_outputs_index(&self, id: usize) -> usize {
        id - self.outputs_start_end_indexes.0
    }

    pub(super) fn get_max_gate_id(&self) -> usize {
        self.max_gate_id
    }
}

#[cfg(test)]
#[derive(Debug, snafu::Snafu)]
pub enum EvaluateError {
    Unknown,
}

#[cfg(test)]
impl Circuit {
    pub(crate) fn num_evaluator_inputs(&self) -> usize {
        let mut num_inputs = 0usize;
        for skcd_input in &self.config.evaluator_inputs {
            num_inputs += skcd_input.length as usize;
        }

        num_inputs
    }

    fn num_garbler_inputs(&self) -> usize {
        let mut num_inputs = 0usize;
        for skcd_input in &self.config.garbler_inputs {
            num_inputs += skcd_input.length as usize;
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
                    Some(GateTypeBinary::XOR) => circuit.xor(
                        bdd_map
                            .get(&input_a.id)
                            .expect("GateType::XOR missing input a!")
                            .clone(),
                        bdd_map
                            .get(&input_b.id)
                            .expect("GateType::XOR missing input b!")
                            .clone(),
                    ),
                    Some(GateTypeBinary::XNOR) => {
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
                    Some(GateTypeBinary::NAND) => {
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
                    Some(GateTypeBinary::NOR) => {
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
                    Some(GateTypeBinary::AND) => circuit.and(
                        bdd_map
                            .get(&input_a.id)
                            .expect("GateType::AND missing input a!")
                            .clone(),
                        bdd_map
                            .get(&input_b.id)
                            .expect("GateType::AND missing input b!")
                            .clone(),
                    ),
                    Some(GateTypeBinary::OR) => circuit.or(
                        bdd_map
                            .get(&input_a.id)
                            .expect("GateType::OR missing input a!")
                            .clone(),
                        bdd_map
                            .get(&input_b.id)
                            .expect("GateType::OR missing input b!")
                            .clone(),
                    ),
                    None => unimplemented!("eval_plain: None GateTypeBinary!"),
                },
                GateType::Unary {
                    gate_type: r#type,
                    input_a,
                } => match r#type {
                    Some(GateTypeUnary::INV) => circuit.not(
                        bdd_map
                            .get(&input_a.id)
                            .expect("GateType::NOT missing input a!")
                            .clone(),
                    ),
                    // ite = If-Then-Else
                    // we define BUF as "if input == 1 then input; else 0"
                    Some(GateTypeUnary::BUF) => circuit.ite(
                        bdd_map
                            .get(&input_a.id)
                            .expect("GateType::NOT missing input a!")
                            .clone(),
                        BDD_ONE,
                        BDD_ZERO,
                    ),
                    None => unimplemented!("eval_plain: None GateTypeUnary!"),
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
                inputs: vec![WireRef { id: 0 }, WireRef { id: 1 }],
                outputs: vec![WireRef { id: 2 }],
                gates: vec![Gate {
                    internal: GateType::Binary {
                        gate_type: Some(gate_binary_type),
                        input_a: WireRef { id: 0 },
                        input_b: WireRef { id: 1 },
                    },
                    output: WireRef { id: 2 },
                }],
                wires: vec![WireRef { id: 0 }, WireRef { id: 1 }, WireRef { id: 2 }],
            },
            config: SkcdConfig {
                display_config: None,
                garbler_inputs: vec![],
                evaluator_inputs: vec![],
            },
            metadata: CircuitMetadata {
                outputs_start_end_indexes: (2, 2),
                gates_unary_count: HashMap::new(),
                gates_binary_count: HashMap::new(),
                max_gate_id: 2,
            },
        }
    }

    pub(crate) fn new_test_circuit_unary(gate_unary_type: GateTypeUnary) -> Self {
        Self {
            circuit: CircuitInternal {
                inputs: vec![WireRef { id: 0 }],
                outputs: vec![WireRef { id: 1 }],
                gates: vec![Gate {
                    internal: GateType::Unary {
                        gate_type: Some(gate_unary_type),
                        input_a: WireRef { id: 0 },
                    },
                    output: WireRef { id: 1 },
                }],
                wires: vec![WireRef { id: 0 }, WireRef { id: 1 }],
            },
            config: SkcdConfig {
                display_config: None,
                garbler_inputs: vec![],
                evaluator_inputs: vec![],
            },
            metadata: CircuitMetadata {
                outputs_start_end_indexes: (1, 1),
                gates_unary_count: HashMap::new(),
                gates_binary_count: HashMap::new(),
                max_gate_id: 1,
            },
        }
    }

    pub(crate) fn new_test_circuit_constant(value: bool) -> Self {
        Self {
            circuit: CircuitInternal {
                inputs: vec![WireRef { id: 0 }],
                outputs: vec![WireRef { id: 1 }],
                gates: vec![Gate {
                    internal: GateType::Constant { value },
                    output: WireRef { id: 1 },
                }],
                wires: vec![WireRef { id: 0 }, WireRef { id: 1 }],
            },
            config: SkcdConfig {
                display_config: None,
                garbler_inputs: vec![],
                evaluator_inputs: vec![],
            },
            metadata: CircuitMetadata {
                outputs_start_end_indexes: (1, 1),
                gates_unary_count: HashMap::new(),
                gates_binary_count: HashMap::new(),
                max_gate_id: 1,
            },
        }
    }
}
