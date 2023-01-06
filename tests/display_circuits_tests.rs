use rand::distributions::Uniform;
use rand::thread_rng;
use std::time::Instant;

mod common;
use crate::common::garble_and_eval_utils::{
    eval_client, garble_display_message_2digits, read_png_to_bytes, write_png,
};
use lib_garble_rs::garbled_display_circuit_prepare_garbler_inputs;

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
    let encoded_garbler_inputs = garbled_display_circuit_prepare_garbler_inputs(&garb, "");

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
    let eval_outputs = write_png(width, height, merged_outputs);

    let expected_outputs = read_png_to_bytes(include_bytes!(
        "../examples/data/eval_outputs_display_message_120x52_2digits_42.png"
    ));
    assert_eq!(eval_outputs, expected_outputs);
}

#[test]
fn test_garble_display_message_120x52_2digits_zeros() {
    let (mut garb, width, height) = garble_display_message_2digits(include_bytes!(
        "../examples/data/display_message_120x52_2digits.skcd.pb.bin"
    ));
    let encoded_garbler_inputs = garb.encode_garbler_inputs(&[0; 1 + 2 * 7 + 120 * 52]);
    let data = garb.eval(&encoded_garbler_inputs, &[0; 9]).unwrap();
    let eval_outputs = write_png(width, height, data);

    let expected_outputs = read_png_to_bytes(include_bytes!(
        "../examples/data/eval_outputs_display_message_120x52_2digits_inputs0.png"
    ));
    assert_eq!(eval_outputs, expected_outputs);
}

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
    const NB_EVALS: usize = 50;

    let mut eval_times = Vec::with_capacity(NB_EVALS);
    // To try and make sure eval is run and NOT optimized-out
    let mut eval_datas = Vec::with_capacity(NB_EVALS);

    let (mut garb, width, height) = garble_display_message_2digits(include_bytes!(
        "../examples/data/display_message_640x360_2digits.skcd.pb.bin"
    ));

    let encoded_garbler_inputs = garbled_display_circuit_prepare_garbler_inputs(&garb, "");

    let mut rng = thread_rng();
    let rand_0_1 = Uniform::from(0..=1);

    let mut evaluator_inputs = vec![
        // "rnd": 9 inputs
        0u16, 0, 0, 0, 0, 0, 0, 0, 0, //
    ];

    let mut data = vec![Some(0u16); width * height];
    let mut eval_cache = garb.init_cache();

    for _ in 0..NB_EVALS {
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

        let duration = start.elapsed();

        eval_times.push(duration.as_millis());
        eval_datas.push(data.iter().filter(|&o| *o != Some(0u16)).count());
    }

    println!("eval_times : {:?}", eval_times);
    println!("eval_datas : {:?}", eval_datas.len());

    // let eval_outputs = write_png(width, height, data);

    // TODO!!! assert? or keep it as a bench?
    // let expected_outputs = read_png_to_bytes(include_bytes!(
    //     "../examples/data/eval_outputs_display_message_640x360_2digits_42.png"
    // ));
    // assert_eq!(eval_outputs, expected_outputs);
}
