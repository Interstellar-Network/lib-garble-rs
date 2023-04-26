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
    pub(crate) outputs: Vec<WireRef>,
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
#[derive(Debug, snafu::Snafu)]
pub enum EvaluateError {
    Unknown,
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
    ///
    /// NOTE: "expected_outputs" are passed as param b/c of the way "evaluate" from the crate "boolean_expression" works
    /// See also: https://stackoverflow.com/questions/59109453/how-do-i-use-the-rust-crate-boolean-expression-to-implement-a-simple-logic-cir
    pub(crate) fn eval_plain(
        &self,
        evaluator_inputs: &[u16],
        expected_outputs: &[u16],
    ) -> Result<(), EvaluateError> {
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
        let mut bdd_map = HashMap::new();

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

        ////////////////////////////////////////////////////////////////////////

        let mut circuit = circuit.clone();

        // bdd.terminal(t)

        // let variables: Vec<String> = bdd.variables().into_iter().map(|v| v.to_string()).collect();

        // cf boolean_expression examples/tests for how the evaluation works
        // https://github.com/cfallin/boolean_expression/blob/795b89567e05f54907b89453bdd481d0b01f0c93/src/bdd.rs#L1071
        // let mut inputs_hashmap = HashMap::new();
        // h.insert("A".to_owned(), true);
        // h.insert("B".to_owned(), true);
        // let hashmap_inputs = evaluator_inputs
        //     .iter()
        //     .enumerate()
        //     .map(|(idx, input)| (idx, input.clone() == 1))
        //     .collect();

        /////////////////////////////////////////////////////////////////////////
        // phind.com explanation of "evaluate":
        //         # Answer
        // The sat_one function in the boolean_expression crate is used to find a satisfying assignment for a given Boolean expression. It returns an Option<HashMap<String, bool>>, where the HashMap contains variable assignments that satisfy the given expression, or None if no such assignment exists.
        // The and and not operations are used to create a combined expression that represents the input conditions and the desired output conditions. In the example, we're trying to find an assignment for the variables A, B, and Cin that satisfies the following conditions:
        //
        //     A is true
        //     B is false
        //     Cin is true
        //     Sum and Cout are the outputs of the full adder for the given input values
        //
        // To create this combined expression, we use the and operation to combine the input conditions:
        //
        // let input_expr = bdd.and(bdd.and(a, bdd.not(b)), cin);
        //
        // Here, a represents A being true, bdd.not(b) represents B being false, and cin represents Cin being true. The and operation combines these conditions, so all of them must be true for the combined expression to be true.
        //
        // Next, we use the and operation again to combine the input conditions with the output expressions (sum and cout):
        //
        // let combined_expr = bdd.and(bdd.and(input_expr, sum), cout);
        //
        // Now, the combined_expr represents an expression that is true only if all the input conditions and the output expressions are true.
        //
        // Finally, we call the sat_one function with the combined_expr to find a satisfying assignment for the combined expression:
        //
        // let all_output = bdd.sat_one(combined_expr);
        //
        // The all_output variable will contain an Option<HashMap<String, bool>> with the variable assignments that satisfy the combined_expr, or None if no such assignment exists. We can then extract the values for Sum and Carry-out from the all_output variable and compare them with the expected set of outputs.
        //
        /////////////////////////////////////////////////////////////////////////
        // So we simply need to AND every INPUTS and OUTPUTS
        // For each "evaluator_inputs" and "expected_outputs" false
        // -> first NOT it, then AND we the current expr

        // let mut expected_expr = circuit.terminal(0);
        // for (idx, input) in evaluator_inputs.iter().enumerate().skip(1) {
        //     let input_term = circuit.terminal(idx);
        //     match input {
        //         1 => expected_expr = circuit.and(input_term, expected_expr),
        //         0 => {
        //             expected_expr = {
        //                 let input_not = circuit.not(input_term);
        //                 circuit.and(input_not, expected_expr)
        //             }
        //         }
        //         _ => unimplemented!("invalid input!"),
        //     }
        // }

        assert!(
            self.circuit.outputs.len() == expected_outputs.len(),
            "outputs len mismatch!"
        );

        // init the with a constant 1
        // that the loop on outputs, which AND everything, will work
        let mut expected_expr = circuit.constant(true);

        for (idx, output) in expected_outputs.iter().enumerate() {
            let output_bddfunc = bdd_map
                .get(&self.circuit.outputs[idx].id)
                .expect("missing output!")
                .clone();
            match output {
                1 => expected_expr = circuit.and(output_bddfunc, expected_expr),
                0 => {
                    expected_expr = {
                        let output_bddfunc_not = circuit.not(output_bddfunc);
                        circuit.and(output_bddfunc_not, expected_expr)
                    }
                }
                _ => unimplemented!("invalid output!"),
            }
        }

        // println!(
        //     "########### evaluate : {}",
        //     circuit.evaluate(expected_expr, &hashmap_inputs)
        // );

        // println!(
        //     "########### to_expr evaluate : {}",
        //     circuit.to_expr(expected_expr).evaluate(&hashmap_inputs)
        // );

        let res = circuit.sat_one(expected_expr);
        // println!("########### sat_one res : {:?}", res);
        let computed_inputs: Vec<bool> = res.unwrap().into_values().collect();
        println!(
            "########### sat_one computed input : {:?}, expected_outputs : {:?}",
            computed_inputs, expected_outputs
        );

        // TODO should reverse inputs? or some other?
        // NO!
        // This seem to work as intented; BUT the thing is for a given value in
        // FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS
        // There are potentially multiple valid set of inputs...
        // and this return one of them
        assert_eq!(
            computed_inputs,
            evaluator_inputs
                .iter()
                .map(|input| input.clone() == 1)
                .collect::<Vec<_>>()
        );

        Ok(())

        ////////////////////////////////////////////////////////////////////////
        // circuit.to_expr(f)
        // circuit.co

        // Convert the BDD back to a CubeList
        // let cubelist = CubeList::from(&bdd);

        // Convert the CubeList to an Expr
        // let expr_from_bdd = Self::cubelist_to_expr(&cubelist);

        // let sum_output = circuit.sat_one_restrict(sum, &hashmap_inputs);
        // let cout_output = circuit.sat_one_restrict(cout, &hashmap_inputs);

        // let restricted_bdd = circuit.restrict(&expected_expr);
        ////////////////////////////////////////////////////////////////////////

        // let mut bdd = circuit.clone();

        // let a = 0;
        // let b = 1;
        // let cin = 2;

        // let sum = self.circuit.outputs[0].id;
        // let cout = self.circuit.outputs[1].id;

        // // let all_output = circuit
        // //     .sat_one(circuit.and(circuit.and(sum, cout), circuit.and(a, circuit.not(b), cin)));

        // let input_expr = bdd.and(bdd.and(a, bdd.not(b)), cin);
        // let combined_expr = bdd.and(bdd.and(input_expr, sum), cout);

        // let all_output = bdd.sat_one(combined_expr);
    }
}
