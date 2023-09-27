//! Run with eg:
//! - `cargo run --example=classic -- --digits=0,1,2,3,4,5,6,7,8,9 --skcd-path=lib-garble-rs/examples/data/display_pinpad_590x50.skcd.postcard.bin --tx-msg-str=""`
//! - `cargo run --example=classic -- --digits=4,2 --skcd-path=lib-garble-rs/examples/data/display_message_640x360_2digits.skcd.postcard.bin --tx-msg-str="abcdefghijklmnopqrstuvwxyz"`
//!
use clap::Parser;
use rand::distributions::Uniform;
use rand::prelude::Distribution;
use rand::thread_rng;
use std::io::BufReader;
use std::io::Read;

use lib_garble_rs::EvalCache;
use lib_garble_rs::{
    garble_skcd, garbled_display_circuit_prepare_garbler_inputs, prepare_evaluator_inputs,
};
use png_utils::write_png;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The tx message; will be written as "watermark"
    #[clap(long, default_value = "test message")]
    tx_msg_str: String,

    /// The digits; ie the OTP or pinpad
    #[clap(
        long,
        required = true,
        num_args = 1..,
        value_delimiter = ','
    )]
    digits: Vec<u8>,

    /// Path to the INPUT .skcd
    #[clap(long)]
    skcd_path: String,

    /// Path to the OUTPUT .garbled
    #[clap(long, default_value = "output.garbled")]
    garbled_path: String,

    /// The seed passed to ChaChaRng
    /// Useful to have repeatable outputs; eg golden tests
    /// NOTE: passed via `seed_from_u64` for simplicity so NOT secure!
    #[clap(long, required = false)]
    rng_seed: Option<u64>,

    /// How many eval() we will combine
    /// Reminder: each segment have a 50% chance to be displayed at each eval()
    /// So typically using 10 evals means almost all of the segments will be displayed
    #[clap(long, required = false, default_value_t = 5)]
    nb_evals: u64,
}

fn main() {
    let args = Args::parse();

    let f = std::fs::File::open(&args.skcd_path).unwrap();
    let mut reader = BufReader::new(f);

    let mut buffer = Vec::new();
    // read the whole file
    reader.read_to_end(&mut buffer).unwrap();

    let garb = garble_skcd(&buffer).unwrap();

    let display_config = garb.get_display_config().unwrap();
    let width = display_config.width as usize;
    let height = display_config.height as usize;

    let mut merged_outputs = vec![0u8; width * height];
    let mut temp_outputs = vec![0u8; width * height];
    let mut eval_cache = EvalCache::new();
    let mut rng = thread_rng();
    let rand_0_1 = Uniform::from(0..=1);

    let mut encoded_garbler_inputs =
        garbled_display_circuit_prepare_garbler_inputs(&garb, &args.digits, &args.tx_msg_str)
            .unwrap();

    let mut evaluator_inputs = prepare_evaluator_inputs(&garb).unwrap();

    for _ in 0..args.nb_evals {
        // randomize the "rnd" part of the inputs
        // cf "rndswitch.v" comment above; DO NOT touch the last!
        for input in evaluator_inputs.iter_mut() {
            *input = rand_0_1.sample(&mut rng);
        }

        garb.eval(
            &mut encoded_garbler_inputs,
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
            *merged_output = std::cmp::min(*merged_output + cur_output, 1u8)
        }
    }

    // convert (0,1) -> (0,255) to get a proper png
    for merged_output in merged_outputs.iter_mut() {
        *merged_output = *merged_output * 255;
    }

    write_png("eval_outputs.png", width, height, &merged_outputs);
}
