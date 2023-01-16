/// Test the client-side use case, or as close as possible:
/// - [server 1] server garbles a circuit
/// - [server 2] server prepares a "watermark" and encode the "garbler_inputs"
/// - [server 3] server serializes all the above
/// - [client 1] client receives those
/// - [client 2] client prepare their own inputs(random)
/// - [client 3] client eval the garbled circuit
use rand::distributions::Uniform;
use rand::thread_rng;

mod common;
use crate::common::garble_and_eval_utils::{eval_client, garble_skcd_helper};
use lib_garble_rs::{garbled_display_circuit_prepare_garbler_inputs, prepare_evaluator_inputs};
use png_tests_utils::png_utils::{convert_vec_u16_to_u8, read_png_to_bytes};

#[test]
fn test_server_client_display_message_120x52_2digits_zeros() {
    let (mut garb, encoded_garbler_inputs) = {
        // [server 1]
        let (garb, _width, _height) = garble_skcd_helper(include_bytes!(
            "../examples/data/display_message_120x52_2digits.skcd.pb.bin"
        ));

        // [server 2]
        let encoded_garbler_inputs =
            garbled_display_circuit_prepare_garbler_inputs(&garb, &[4, 2], "").unwrap();

        // TODO [server 3]
        (garb, encoded_garbler_inputs)
    };

    let eval_outputs = {
        // TODO [client 1]
        let width = garb.config.display_config.unwrap().width as usize;
        let height = garb.config.display_config.unwrap().height as usize;

        let mut rng = thread_rng();
        let rand_0_1 = Uniform::from(0..=1);

        let mut outputs = vec![Some(0u16); width * height];

        // [client 2]
        let mut evaluator_inputs = prepare_evaluator_inputs(&garb).unwrap();

        let mut eval_cache = garb.init_cache();

        // [client 3]
        eval_client(
            &mut garb,
            &encoded_garbler_inputs,
            &mut evaluator_inputs,
            &mut outputs,
            &mut rng,
            &rand_0_1,
            &mut eval_cache,
            false,
        );

        // convert Vec<std::option::Option<u16>> -> Vec<u16>
        let outputs: Vec<u16> = outputs.into_iter().map(|i| i.unwrap()).collect();
        // convert Vec<u16> -> Vec<u8>
        let outputs = convert_vec_u16_to_u8(&outputs);

        outputs
    };

    let expected_outputs = read_png_to_bytes(include_bytes!(
        "../examples/data/eval_outputs_display_message_120x52_2digits_inputs0.png"
    ));
    assert_eq!(eval_outputs, expected_outputs);
}
