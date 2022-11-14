use fancy_garbling::classic::garble;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;

use lib_garble_rs::skcd_parser::parse_skcd;

fn main() {
    ////////////////////////////////////////////////////////////////////////////

    if true {
        let f = std::fs::File::open("examples/data/adder.skcd.pb.bin").unwrap();
        let mut reader = BufReader::new(f);

        let mut buffer = Vec::new();
        // read the whole file
        reader.read_to_end(&mut buffer).unwrap();

        let circ = parse_skcd(&buffer).unwrap();

        // all_inputs/all_expected_outputs: standard full-adder 2 bits truth table(and expected results)
        // input  i_bit1;
        // input  i_bit2;
        // input  i_carry;
        let all_inputs = vec![
            [0, 0, 0],
            [1, 0, 0],
            [0, 1, 0],
            [1, 1, 0],
            [0, 0, 1],
            [1, 0, 1],
            [0, 1, 1],
            [1, 1, 1],
        ];

        // output o_sum;
        // output o_carry;
        let all_expected_outputs = [
            [0, 0],
            [1, 0],
            [1, 0],
            [0, 1],
            [1, 0],
            [0, 1],
            [0, 1],
            [1, 1],
        ];

        assert!(circ.num_evaluator_inputs() == 3);
        for (i, inputs) in all_inputs.iter().enumerate() {
            let outputs = circ.eval_plain(&[], inputs).unwrap();
            if outputs == all_expected_outputs[i] {
                println!("adder OK");
            } else {
                println!("adder FAIL!");
            }
        }
    }

    //////////////////////////////////
    // TODO refactor "adder" as a test; and then add version with "display" and then write .png

    let f =
        std::fs::File::open("examples/data/display_message_120x52_2digits.skcd.pb.bin").unwrap();
    let mut reader = BufReader::new(f);

    let mut buffer = Vec::new();
    // read the whole file
    reader.read_to_end(&mut buffer).unwrap();

    let circ = parse_skcd(&buffer).unwrap();

    let outputs = circ.eval_plain(&[], &[1; 24]).unwrap();

    let path = "eval_outputs.png";
    let file = std::fs::File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);

    // TODO(interstellar) get from Circuit's "config"
    let mut encoder = png::Encoder::new(w, 120, 52);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().unwrap();

    // let data = [255, 0, 0, 255, 0, 0, 0, 255]; // "An array containing a RGBA sequence. First pixel is red and second pixel is black."
    let data: Vec<u8> = outputs
        .iter()
        .map(|v| {
            let pixel_value: u8 = (*v).try_into().unwrap();
            pixel_value * 255
        })
        .collect();

    // TODO(interstellar) FIX: nb outputs SHOULD be == 120x52 = 6240; but 6341 for now!
    // possibly linked to  println!("output called"); in fancy-garbling/src/circuit.rs ?
    writer.write_image_data(&data).unwrap(); // Save

    //////////////////////////////////
}
