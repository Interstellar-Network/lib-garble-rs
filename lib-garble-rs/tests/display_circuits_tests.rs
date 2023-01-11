use rand::distributions::Uniform;
use rand::thread_rng;
use std::time::Instant;

mod common;
use crate::common::garble_and_eval_utils::{eval_client, garble_display_message_2digits};
use lib_garble_rs::garbled_display_circuit_prepare_garbler_inputs;
use tests_utils::png_utils::{convert_vec_u16_to_u8, read_png_to_bytes};

// TODO!!! MUST combine multiple evals; or alternatively have several tests with different "evaluator_inputs"
#[test]
fn test_garble_display_message_120x52_2digits_42() {
    // The more we combine, the less this test will be flaky
    // TODO should we instead map "specific inputs" -> "expected outputs"; and assume everything is OK is eg 10 random inputs are OK?
    const NB_EVALS: usize = 50;

    let (mut garb, width, height) = garble_display_message_2digits(include_bytes!(
        "../examples/data/display_message_120x52_2digits.skcd.pb.bin"
    ));

    let mut merged_outputs = vec![0u16; width * height];
    let mut rng = thread_rng();
    let rand_0_1 = Uniform::from(0..=1);

    let mut temp_outputs = vec![Some(0u16); width * height];

    let mut evaluator_inputs = vec![
        // "rnd": 9 inputs
        0u16, 0, 0, 0, 0, 0, 0, 0, 0, //
    ];
    let encoded_garbler_inputs =
        garbled_display_circuit_prepare_garbler_inputs(&garb, &[4, 2], "").unwrap();

    let mut eval_cache = garb.init_cache();

    for _ in 0..NB_EVALS {
        eval_client(
            &mut garb,
            &encoded_garbler_inputs,
            &mut evaluator_inputs,
            &mut temp_outputs,
            &mut rng,
            &rand_0_1,
            &mut eval_cache,
            true,
        );

        for (merged_output, &cur_output) in merged_outputs.iter_mut().zip(temp_outputs.iter()) {
            // what we want is a OR:
            // 0 + 0 = 0
            // 1 + 0 = 1
            // 0 + 1 = 1
            // 1 + 1 = 1
            *merged_output = std::cmp::min(*merged_output + cur_output.unwrap_or_default(), 1u16)
        }
    }

    let merged_outputs = convert_vec_u16_to_u8(&merged_outputs);
    let expected_outputs = read_png_to_bytes(include_bytes!(
        "../examples/data/eval_outputs_display_message_120x52_2digits_42.png"
    ));
    assert_eq!(merged_outputs, expected_outputs);
}

#[test]
fn test_garble_display_message_120x52_2digits_zeros() {
    let (mut garb, _width, _height) = garble_display_message_2digits(include_bytes!(
        "../examples/data/display_message_120x52_2digits.skcd.pb.bin"
    ));
    let encoded_garbler_inputs =
        garbled_display_circuit_prepare_garbler_inputs(&garb, &[4, 2], "").unwrap();
    let evaluator_inputs = vec![0u16; 9];
    let width = garb.config.display_config.unwrap().width as usize;
    let height = garb.config.display_config.unwrap().height as usize;
    let mut outputs = vec![Some(0u16); width * height];
    let mut eval_cache = garb.init_cache();
    garb.eval_with_prealloc(
        &encoded_garbler_inputs,
        &evaluator_inputs,
        &mut outputs,
        &mut eval_cache,
    )
    .unwrap();
    // convert Vec<std::option::Option<u16>> -> Vec<u16>
    let outputs: Vec<u16> = outputs.into_iter().map(|i| i.unwrap()).collect();
    // convert Vec<u16> -> Vec<u8>
    let outputs = convert_vec_u16_to_u8(&outputs);

    let expected_outputs = read_png_to_bytes(include_bytes!(
        "../examples/data/eval_outputs_display_message_120x52_2digits_inputs0.png"
    ));
    assert_eq!(outputs, expected_outputs);
}

/// BENCH "eval_with_prealloc"
/// ie the client-side of the pipeline
// NOTE it is quite slow in Debug! Make sure to enable optimizations
#[test]
fn bench_eval_display_message_640x360_2digits_42() {
    ////////////////////////////////////////////////////////////////////////
    // use tracing_subscriber::layer::SubscriberExt;

    // #[cfg(feature = "profile-with-tracy")]
    // let _client = tracy_client::Client::start();

    // tracing::subscriber::set_global_default(
    //     tracing_subscriber::registry().with(tracing_tracy::TracyLayer::new()),
    // )
    // .expect("set up the subscriber");

    // profiling::register_thread!("Main Thread");
    ////////////////////////////////////////////////////////////////////////

    // coz::thread_init();

    //////////////////////////////////////////////////////////////////////

    // How many eval() we will combine
    // Reminder: each segment have a 50% chance to be displayed at each eval()
    // So typically using 10 evals means almost all of the segments will be displayed
    const NB_ITERATIONS: usize = 50;

    let mut loop_times = Vec::with_capacity(NB_ITERATIONS);

    let (mut garb, width, height) = garble_display_message_2digits(include_bytes!(
        "../examples/data/display_message_640x360_2digits.skcd.pb.bin"
    ));

    let encoded_garbler_inputs =
        garbled_display_circuit_prepare_garbler_inputs(&garb, &[4, 2], "").unwrap();

    let mut rng = thread_rng();
    let rand_0_1 = Uniform::from(0..=1);

    let mut evaluator_inputs = vec![
        // "rnd": 9 inputs
        0u16, 0, 0, 0, 0, 0, 0, 0, 0, //
    ];

    let mut data = vec![Some(0u16); width * height];
    let mut eval_cache = garb.init_cache();

    for _ in 0..NB_ITERATIONS {
        // profiling::scope!("Looped eval");
        // coz::progress!();

        let start = Instant::now();

        eval_client(
            &mut garb,
            &encoded_garbler_inputs,
            &mut evaluator_inputs,
            &mut data,
            &mut rng,
            &rand_0_1,
            &mut eval_cache,
            true,
        );

        core::hint::black_box(&data);
        assert!(data.len() > 0);

        let duration = start.elapsed();

        loop_times.push(duration.as_millis());
    }

    println!("loop_times : {:?}", loop_times);

    // let eval_outputs = write_png(width, height, data);

    // TODO!!! assert? or keep it as a bench?
    // let expected_outputs = read_png_to_bytes(include_bytes!(
    //     "../examples/data/eval_outputs_display_message_640x360_2digits_42.png"
    // ));
    // assert_eq!(eval_outputs, expected_outputs);
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

        let (garb, width, height) = garble_display_message_2digits(include_bytes!(
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
