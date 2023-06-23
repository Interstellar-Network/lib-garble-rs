/// Basic client that mimics the server-side:
/// - parse a .skcd (from a path)
/// - garble it
/// - serializes it
///
/// NOTE: tested ONLY with "display circuits"
///
/// To run:
/// - `cargo run --profile=release-with-debug --example garble_and_serialize -- --skcd-path=./lib-garble-rs/examples/data/display_message_640x360_2digits.skcd.pb.bin --digits=4,2 --garbled-path=message.garbled`
/// - `cargo run --profile=release-with-debug --example garble_and_serialize -- --skcd-path=./lib-garble-rs/examples/data/display_pinpad_590x50.skcd.pb.bin --digits=0,1,2,3,4,5,6,7,8,9 --garbled-path=pinpad.garbled --tx-msg-str=""`
///
///
use std::io::BufReader;
use std::io::Read;
use std::io::Write;

use clap::Parser;

use lib_garble_rs::garble_skcd;
use lib_garble_rs::serialize_for_evaluator;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The tx message; will be written as "watermark"
    #[clap(long, default_value = "test message")]
    tx_msg_str: String,

    /// The digits; ie the OTP or pinpad
    // TODO(clap 4) https://stackoverflow.com/questions/73240901/how-to-get-clap-to-process-a-single-argument-with-multiple-values-without-having
    #[clap(
        long,
        multiple = true,
        required = true,
        use_value_delimiter = true,
        value_delimiter = ','
    )]
    digits: Vec<u8>,

    /// Path to the INPUT .skcd
    #[clap(long)]
    skcd_path: String,

    /// Path to the OUTPUT .garbled
    #[clap(long, default_value = "output.garbled")]
    garbled_path: String,
}

fn main() {
    let args = Args::parse();

    println!("digits: {:?}", args.digits);

    let f = std::fs::File::open(&args.skcd_path).unwrap();
    let mut reader = BufReader::new(f);

    let mut buffer = Vec::new();
    // read the whole file
    reader.read_to_end(&mut buffer).unwrap();

    let mut garb = garble_skcd(&buffer).unwrap();

    // ex-"packsmg"
    let encoded_garbler_inputs = lib_garble_rs::garbled_display_circuit_prepare_garbler_inputs(
        &garb,
        &args.digits,
        &args.tx_msg_str,
    )
    .unwrap();
    // then serialize "garb" and "packmsg"
    let serialized_package_for_eval =
        lib_garble_rs::serialize_for_evaluator(garb, encoded_garbler_inputs).unwrap();

    let mut out = std::fs::File::create(&args.garbled_path).unwrap();
    out.write_all(&serialized_package_for_eval).unwrap();
}
