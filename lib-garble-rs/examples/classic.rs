use rand::distributions::Uniform;
use rand::prelude::Distribution;
use rand::thread_rng;
use std::io::BufReader;
use std::io::Read;

use lib_garble_rs::{
    garble_skcd, garbled_display_circuit_prepare_garbler_inputs, prepare_evaluator_inputs,
};
use tests_utils::png_utils::write_png;

fn main() {
    // How many eval() we will combine
    // Reminder: each segment have a 50% chance to be displayed at each eval()
    // So typically using 10 evals means almost all of the segments will be displayed
    const NB_EVALS: i32 = 10;

    // TODO(interstellar) display_message_640x360_2digits.skcd.pb.bin
    let f = std::fs::File::open("lib-garble-rs/examples/data/display_pinpad_590x50.skcd.pb.bin")
        .unwrap();
    let mut reader = BufReader::new(f);

    let mut buffer = Vec::new();
    // read the whole file
    reader.read_to_end(&mut buffer).unwrap();

    let mut garb = garble_skcd(&buffer).unwrap();

    let display_config = garb.config.display_config.unwrap().clone();
    let width = display_config.width as usize;
    let height = display_config.height as usize;

    let mut merged_outputs = vec![0u16; width * height];
    let mut temp_outputs = vec![Some(0u16); width * height];
    let mut rng = thread_rng();
    let rand_0_1 = Uniform::from(0..=1);

    let encoded_garbler_inputs =
        garbled_display_circuit_prepare_garbler_inputs(&garb, &[0, 1, 2, 9, 8, 7, 6, 5, 4, 3], "")
            .unwrap();

    let mut evaluator_inputs = prepare_evaluator_inputs(&garb).unwrap();

    let mut eval_cache = garb.init_cache();

    for _ in 0..NB_EVALS {
        // randomize the "rnd" part of the inputs
        // cf "rndswitch.v" comment above; DO NOT touch the last!
        for input in evaluator_inputs.iter_mut() {
            *input = rand_0_1.sample(&mut rng);
        }

        garb.eval_with_prealloc(
            &encoded_garbler_inputs,
            &evaluator_inputs,
            &mut temp_outputs,
            &mut eval_cache,
        )
        .unwrap();
        assert_eq!(
            temp_outputs.len(),
            merged_outputs.len(),
            "outputs size mistmatch!"
        );

        for (merged_output, &cur_output) in merged_outputs.iter_mut().zip(temp_outputs.iter()) {
            // what we want is a OR:
            // 0 + 0 = 0
            // 1 + 0 = 1
            // 0 + 1 = 1
            // 1 + 1 = 1
            *merged_output = std::cmp::min(*merged_output + cur_output.unwrap(), 1u16)
        }
    }

    write_png("eval_outputs.png", width, height, &merged_outputs);
}
