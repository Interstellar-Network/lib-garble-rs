use rand::distributions::Uniform;
use rand::thread_rng;
use std::time::Instant;

use lib_garble_rs::{
    garbled_display_circuit_prepare_garbler_inputs, prepare_evaluator_inputs,
    tests_utils::garble_and_eval_utils::eval_client,
    tests_utils::garble_and_eval_utils::garble_skcd_helper, EvalCache,
};
use png_tests_utils::png_utils::read_png_to_bytes;

/// MUST combine multiple evals; or alternatively have several tests with different "evaluator_inputs"
fn garble_and_eval(skcd_bytes: &[u8], digits: &[u8]) -> Vec<u8> {
    // The more we combine, the less this test will be flaky
    // TODO should we instead map "specific inputs" -> "expected outputs"; and assume everything is OK is eg 10 random inputs are OK?
    const NB_EVALS: usize = 50;

    let (mut garb, width, height) = garble_skcd_helper(skcd_bytes);

    let mut merged_outputs = vec![0u8; width * height];
    let mut rng = thread_rng();
    let rand_0_1 = Uniform::from(0..=1);

    let mut temp_outputs = vec![0u8; width * height];
    let mut eval_cache = EvalCache::new();

    let mut encoded_garbler_inputs =
        garbled_display_circuit_prepare_garbler_inputs(&garb, digits, "").unwrap();
    let mut evaluator_inputs = prepare_evaluator_inputs(&garb).unwrap();

    for _ in 0..NB_EVALS {
        eval_client(
            &mut garb,
            &mut encoded_garbler_inputs,
            &mut evaluator_inputs,
            &mut temp_outputs,
            &mut eval_cache,
            &mut rng,
            &rand_0_1,
            true,
        );

        for (merged_output, &cur_output) in merged_outputs.iter_mut().zip(temp_outputs.iter()) {
            // what we want is a OR:
            // 0 + 0 = 0
            // 1 + 0 = 1
            // 0 + 1 = 1
            // 1 + 1 = 1
            *merged_output = std::cmp::min(*merged_output + cur_output, 1u8)
        }
    }

    // Convert Vec<0/1u8> -> Vec<0/255u8>; needed to have a proper png-like image output
    for merged_output in merged_outputs.iter_mut() {
        *merged_output *= 255;
    }

    merged_outputs
}

#[test]
fn test_garble_display_message_120x52_2digits_42() {
    let merged_outputs = garble_and_eval(
        include_bytes!("../examples/data/display_message_120x52_2digits.skcd.pb.bin"),
        &[4, 2],
    );
    let expected_outputs = read_png_to_bytes(include_bytes!(
        "../examples/data/eval_outputs_display_message_120x52_2digits_42.png"
    ));
    assert_eq!(merged_outputs, expected_outputs);
}

#[test]
fn test_garble_display_pinpad_590x50() {
    let merged_outputs = garble_and_eval(
        include_bytes!("../examples/data/display_pinpad_590x50.skcd.pb.bin"),
        &[0, 1, 2, 9, 8, 7, 6, 5, 4, 3],
    );
    let expected_outputs = read_png_to_bytes(include_bytes!(
        "../examples/data/eval_outputs_display_pinpad_590x50.png"
    ));
    assert_eq!(merged_outputs, expected_outputs);
}

#[test]
fn test_garble_display_message_120x52_2digits_zeros() {
    let (garb, _width, _height) = garble_skcd_helper(include_bytes!(
        "../examples/data/display_message_120x52_2digits.skcd.pb.bin"
    ));
    let mut encoded_garbler_inputs =
        garbled_display_circuit_prepare_garbler_inputs(&garb, &[4, 2], "").unwrap();
    let evaluator_inputs = vec![0u8; 9];
    let width = garb.config.display_config.unwrap().width as usize;
    let height = garb.config.display_config.unwrap().height as usize;
    let mut outputs = vec![0u8; width * height];
    let mut eval_cache = EvalCache::new();
    garb.eval(
        &mut encoded_garbler_inputs,
        &evaluator_inputs,
        &mut outputs,
        &mut eval_cache,
    )
    .unwrap();

    let expected_outputs = read_png_to_bytes(include_bytes!(
        "../examples/data/eval_outputs_display_message_120x52_2digits_inputs0.png"
    ));
    assert_eq!(outputs, expected_outputs);
}

/// BENCH "garble" + "garbled_display_circuit_prepare_garbler_inputs"
/// ie the server-side of the pipeline
// NOTE it is quite slow in Debug! Make sure to enable optimizations
#[test]
fn bench_garble_display_message_640x360_2digits_42() {
    const NB_ITERATIONS: usize = 5;

    let mut loop_times = Vec::with_capacity(NB_ITERATIONS);

    for _ in 0..NB_ITERATIONS {
        let start = Instant::now();

        let (garb, width, height) = garble_skcd_helper(include_bytes!(
            "../examples/data/display_message_640x360_2digits.skcd.pb.bin"
        ));

        let encoded_garbler_inputs =
            garbled_display_circuit_prepare_garbler_inputs(&garb, &[4, 2], "aaa\nbbb").unwrap();

        core::hint::black_box(garb);
        core::hint::black_box(width);
        core::hint::black_box(height);
        core::hint::black_box(encoded_garbler_inputs);

        let duration = start.elapsed();

        loop_times.push(duration.as_millis());
    }

    println!("loop_times : {:?}", loop_times);
}
