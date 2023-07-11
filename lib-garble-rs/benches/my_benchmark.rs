use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::distributions::Uniform;
use rand::thread_rng;

use lib_garble_rs::garbled_display_circuit_prepare_garbler_inputs;
use lib_garble_rs::prepare_evaluator_inputs;
use lib_garble_rs::tests_utils::garble_and_eval_utils::eval_client;
use lib_garble_rs::tests_utils::garble_and_eval_utils::garble_skcd_helper;
use lib_garble_rs::EvalCache;

pub fn bench_eval_display_message_640x360_2digits_42(c: &mut Criterion) {
    let (garb, width, height) = garble_skcd_helper(include_bytes!(
        "../examples/data/display_message_640x360_2digits.skcd.pb.bin"
    ));

    let encoded_garbler_inputs =
        garbled_display_circuit_prepare_garbler_inputs(&garb, &[4, 2], "").unwrap();

    let mut rng = thread_rng();
    let rand_0_1 = Uniform::from(0..=1);

    let mut evaluator_inputs = prepare_evaluator_inputs(&garb).unwrap();

    let mut outputs = vec![0u8; width * height];
    let mut eval_cache = EvalCache::new();

    c.bench_function("eval_display_message_640x360_2digits_42", |b| {
        b.iter(|| {
            eval_client(
                black_box(&garb),
                black_box(&encoded_garbler_inputs),
                black_box(&mut evaluator_inputs),
                black_box(&mut outputs),
                black_box(&mut eval_cache),
                black_box(&mut rng),
                black_box(&rand_0_1),
                black_box(true),
            )
        })
    });
}

criterion_group! {
    name = benches;
    // This can be any expression that returns a `Criterion` object.
    // warm_up_time: default is 3s, but re-running the bench causes almost 10% variation on the same machine run after run...
    config = Criterion::default().sample_size(1000).warm_up_time(core::time::Duration::from_millis(6000));
    targets = bench_eval_display_message_640x360_2digits_42
}
criterion_main!(benches);
