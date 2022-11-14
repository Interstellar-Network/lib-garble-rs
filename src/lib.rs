pub mod circuit;
mod skcd_parser;

#[cfg(test)]
mod tests {
    use crate::circuit::InterstellarCircuit;

    #[test]
    fn test_full_adder_2bits() {
        let circ =
            InterstellarCircuit::parse_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin"))
                .unwrap();

        // all_inputs/all_expected_outputs: standard full-adder 2 bits truth table(and expected results)
        // input  i_bit1;
        // input  i_bit2;
        // input  i_carry;
        let all_inputs = vec![
            [0, 0, 0],
            [1, 0, 0],
            [0, 1, 0],
            [1, 1, 0],
            [0, 0, 1],
            [1, 0, 1],
            [0, 1, 1],
            [1, 1, 1],
        ];

        // output o_sum;
        // output o_carry;
        let all_expected_outputs = [
            [0, 0],
            [1, 0],
            [1, 0],
            [0, 1],
            [1, 0],
            [0, 1],
            [0, 1],
            [1, 1],
        ];

        assert!(circ.num_evaluator_inputs() == 3);
        for (i, inputs) in all_inputs.iter().enumerate() {
            let outputs = circ.eval_plain(&[], inputs).unwrap();
            assert_eq!(outputs, all_expected_outputs[i]);
        }
    }
}
