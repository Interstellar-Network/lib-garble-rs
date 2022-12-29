use rand::distributions::Uniform;
use rand::prelude::Distribution;
use rand::thread_rng;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;

use lib_garble_rs::garble_skcd;
use lib_garble_rs::watermark;

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

    let mut garb = garble_skcd(&buffer);

    let display_config = garb.config.display_config.unwrap().clone();
    let width = display_config.width as usize;
    let height = display_config.height as usize;

    let mut merged_outputs = vec![0u16; width * height];
    let mut rng = thread_rng();
    let rand_0_1 = Uniform::from(0..=1);

    let watermark_font = watermark::new_font();
    let watermark = watermark::draw_text(
        width.try_into().unwrap(),
        height.try_into().unwrap(),
        &watermark_font,
        "Hello,\nworld!",
    );

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
    let garbler_input_watermark = watermark::convert_image_to_garbler_inputs(watermark);

    let garbler_inputs = [
        garbler_input_buf.clone(),
        garbler_input_segments.clone(),
        garbler_input_watermark.clone(),
    ]
    .concat();

    let mut evaluator_inputs = vec![
        // "rnd": 9 inputs
        0u16, 0, 0, 0, 0, 0, 0, 0, 0, //
    ];

    for _ in 0..NB_EVALS {
        // randomize the "rnd" part of the inputs
        // cf "rndswitch.v" comment above; DO NOT touch the last!
        for input in evaluator_inputs.iter_mut() {
            *input = rand_0_1.sample(&mut rng);
        }

        let temp_outputs = garb.eval(&garbler_inputs, &evaluator_inputs).unwrap();
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
