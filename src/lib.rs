pub mod circuit;
pub mod garble;
mod skcd_parser;

#[cfg(test)]
mod tests {
    use crate::circuit::InterstellarCircuit;

    // all_inputs/all_expected_outputs: standard full-adder 2 bits truth table(and expected results)
    // input  i_bit1;
    // input  i_bit2;
    // input  i_carry;
    const full_adder_2bits_all_inputs: &'static [&'static [u16]] = &[
        &[0, 0, 0],
        &[1, 0, 0],
        &[0, 1, 0],
        &[1, 1, 0],
        &[0, 0, 1],
        &[1, 0, 1],
        &[0, 1, 1],
        &[1, 1, 1],
    ];

    // output o_sum;
    // output o_carry;
    const full_adder_2bits_all_expected_outputs: &'static [&'static [u16]] = &[
        &[0, 0],
        &[1, 0],
        &[1, 0],
        &[0, 1],
        &[1, 0],
        &[0, 1],
        &[0, 1],
        &[1, 1],
    ];

    #[test]
    fn test_eval_plain_full_adder_2bits() {
        let circ =
            InterstellarCircuit::parse_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin"))
                .unwrap();

        assert!(circ.num_evaluator_inputs() == 3);
        for (i, inputs) in full_adder_2bits_all_inputs.iter().enumerate() {
            let outputs = circ.eval_plain(&[], inputs).unwrap();
            assert_eq!(outputs, full_adder_2bits_all_expected_outputs[i]);
        }
    }

    #[test]
    fn test_garble_full_adder_2bits() {
        use crate::garble::InterstellarGarbledCircuit;

        let circ =
            InterstellarCircuit::parse_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin"))
                .unwrap();

        let garb = InterstellarGarbledCircuit::garble(circ);

        for (i, inputs) in full_adder_2bits_all_inputs.iter().enumerate() {
            let outputs = garb.eval(inputs, &[]).unwrap();
            assert_eq!(outputs, full_adder_2bits_all_expected_outputs[i]);
        }
    }
}
