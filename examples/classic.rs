use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;

use lib_garble_rs::circuit::InterstellarCircuit;

fn main() {
    //////////////////////////////////
    // TODO refactor "adder" as a test; and then add version with "display" and then write .png

    let f =
        std::fs::File::open("examples/data/display_message_120x52_2digits.skcd.pb.bin").unwrap();
    let mut reader = BufReader::new(f);

    let mut buffer = Vec::new();
    // read the whole file
    reader.read_to_end(&mut buffer).unwrap();

    let circ = InterstellarCircuit::parse_skcd(&buffer).unwrap();

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
