use rand::distributions::Uniform;
use rand::prelude::Distribution;
use rand::thread_rng;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;

use lib_garble_rs::circuit::InterstellarCircuit;
use lib_garble_rs::garble::InterstellarGarbledCircuit;

fn main() {
    // How many eval() we will combine
    // Reminder: each segment have a 50% chance to be displayed at each eval()
    // So typically using 10 evals means almost all of the segments will be displayed
    const NB_EVALS: i32 = 10;

    // TODO(interstellar) display_message_640x360_2digits.skcd.pb.bin
    let f =
        std::fs::File::open("examples/data/display_message_640x360_2digits.skcd.pb.bin").unwrap();
    let mut reader = BufReader::new(f);

    let mut buffer = Vec::new();
    // read the whole file
    reader.read_to_end(&mut buffer).unwrap();

    let circ = InterstellarCircuit::parse_skcd(&buffer).unwrap();

    let mut garb = InterstellarGarbledCircuit::garble(circ);

    // TODO(interstellar) get from Circuit's "config"
    let width = 640;
    let height = 360;
    // let width = 120;
    // let height = 52;

    let mut merged_outputs = vec![0u16; width * height];
    let mut rng = thread_rng();
    let rand_0_1 = Uniform::from(0..=1);

    for _ in 0..NB_EVALS {
        // TODO(interstellar)!!! BROKEN CIRCUIT??? no way to properly display a given digit?
        let mut evaluator_inputs = vec![
            // cf Verilog "rndswitch.v"; and correspondingly lib_garble/src/packmsg/packmsg_utils.cpp PrepareInputLabels
            // It MUST always be 0; else the segments will not be displayed properly
            0, //
            // first digit: 7 segments: 4
            0, 1, 1, 1, 0, 1, 0, //
            // second digit: 7 segments: 2
            1, 0, 1, 1, 1, 0, 1, //
            // "rnd": 10 inputs; value SHOULD not really matter for this test???
            0, 0, 0, 0, 0, 0, 0, 0, 0, //
        ];

        // randomize the "rnd" part of the inputs
        let size = evaluator_inputs.len();
        // cf "rndswitch.v" comment above; DO NOT touch the last!
        for input in evaluator_inputs[14 + 1..size].iter_mut() {
            *input = rand_0_1.sample(&mut rng);
        }

        let temp_outputs = garb.eval(&[], &evaluator_inputs).unwrap();
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
            *merged_output = std::cmp::min(*merged_output + cur_output, 1u16)
        }
    }

    let path = "eval_outputs.png";
    let file = std::fs::File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width.try_into().unwrap(), height.try_into().unwrap());
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().unwrap();

    // let data = [255, 0, 0, 255, 0, 0, 0, 255]; // "An array containing a RGBA sequence. First pixel is red and second pixel is black."
    let data: Vec<u8> = merged_outputs
        .iter()
        .map(|v| {
            let pixel_value: u8 = (*v).try_into().unwrap();
            pixel_value * 255
        })
        .collect();

    // TODO(interstellar) FIX: nb outputs SHOULD be == 120x52 = 6240; but 6341 for now!
    // possibly linked to  println!("output called"); in fancy-garbling/src/circuit.rs ?
    writer.write_image_data(&data).unwrap(); // Save
}
