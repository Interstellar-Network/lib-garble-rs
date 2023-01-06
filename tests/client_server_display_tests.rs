/// Test the client-side use case, or as close as possible:
/// - [server 1] server garbles a circuit
/// - [server 2] server prepares a "watermark" and encode the "garbler_inputs"
/// - [server 3] server serializes all the above
/// - [client 1] client receives those
/// - [client 2] client prepare their own inputs(random)
/// - [client 3] client eval the garbled circuit
use rand::distributions::Uniform;
use rand::thread_rng;
use std::time::Instant;

mod common;
use crate::common::garble_and_eval_utils::{
    eval_client, garble_display_message_2digits, read_png_to_bytes, write_png,
};

#[test]
fn test_server_client_display_message_120x52_2digits_zeros() {
    let (mut garb, encoded_garbler_inputs) = {
        // [server 1]
        let (mut garb, _width, _height) = garble_display_message_2digits(include_bytes!(
            "../examples/data/display_message_120x52_2digits.skcd.pb.bin"
        ));

        // TODO this should expose via a "pub fn" somewhere
        let garbler_inputs = {
            // TODO proper garbler inputs
            // Those are splitted into:
            // - "buf" gate (cf Verilog "rndswitch.v"; and correspondingly lib_garble/src/packmsg/packmsg_utils.cpp PrepareInputLabels);
            //    it MUST always be 0 else the 7 segments will not work as expected = 1 bit
            // - the segments to display: 7 segments * "nb of digits in the message" = 7 * N bits
            // - the watermark; one bit per pixel in the final display = width * height bits
            let garbler_input_buf = vec![0u16];
            let garbler_input_segments = vec![
                // first digit: 7 segments: 4
                0u16, 1, 1, 1, 0, 1, 0, //
                // second digit: 7 segments: 2
                1u16, 0, 1, 1, 1, 0, 1, //
            ];
            let garbler_input_watermark = vec![0u16; 120 * 52];

            let garbler_inputs = [
                garbler_input_buf.clone(),
                garbler_input_segments.clone(),
                garbler_input_watermark.clone(),
            ]
            .concat();

            garbler_inputs
        };

        // [server 2]
        let encoded_garbler_inputs = garb.encode_garbler_inputs(&garbler_inputs);

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
        let mut evaluator_inputs = vec![
            // "rnd": 9 inputs
            0u16, 0, 0, 0, 0, 0, 0, 0, 0, //
        ];

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
        let outputs = outputs.into_iter().map(|i| i.unwrap()).collect();

        write_png(width, height, outputs)
    };

    let expected_outputs = read_png_to_bytes(include_bytes!(
        "../examples/data/eval_outputs_display_message_120x52_2digits_inputs0.png"
    ));
    assert_eq!(eval_outputs, expected_outputs);
}
