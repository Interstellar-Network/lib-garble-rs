use lib_garble_rs::garble_skcd;
use lib_garble_rs::EncodedGarblerInputs;
use lib_garble_rs::EvaluatorInput;
use lib_garble_rs::InterstellarGarbledCircuit;

use rand::distributions::Uniform;
use rand::prelude::Distribution;
use rand::rngs::ThreadRng;

/// Client use-case, or as close as possible.
/// Randomize "evaluator_inputs" every call.
///
/// NOT using the "standard API" b/c that re-encodes teh garbler_inputs every eval
/// That costs around ~5ms...
/// let data = garb.eval(&garbler_inputs, &[0; 9]).unwrap();
// #[profiling::function]
pub fn eval_client(
    garb: &mut InterstellarGarbledCircuit,
    encoded_garbler_inputs: &EncodedGarblerInputs,
    evaluator_inputs: &mut [EvaluatorInput],
    data: &mut Vec<Option<u16>>,
    rng: &mut ThreadRng,
    rand_0_1: &Uniform<u16>,
    eval_cache: &mut lib_garble_rs::EvalCache,
    should_randomize_evaluator_inputs: bool,
) {
    // randomize the "rnd" part of the inputs
    // cf "rndswitch.v" comment above; DO NOT touch the last!
    if should_randomize_evaluator_inputs {
        for input in evaluator_inputs.iter_mut() {
            *input = rand_0_1.sample(rng);
        }
    }

    // coz::scope!("eval_client");

    garb.eval_with_prealloc(encoded_garbler_inputs, evaluator_inputs, data, eval_cache)
        .unwrap();
}

/// cf https://docs.rs/png/latest/png/#using-the-decoder
pub fn read_png_to_bytes(buf: &[u8]) -> Vec<u8> {
    // The decoder is a build for reader and can be used to set various decoding options
    // via `Transformations`. The default output transformation is `Transformations::IDENTITY`.
    let decoder = png::Decoder::new(buf);
    let mut reader = decoder.read_info().unwrap();
    // Allocate the output buffer.
    let mut buf = vec![0; reader.output_buffer_size()];
    // Read the next frame. An APNG might contain multiple frames.
    let info = reader.next_frame(&mut buf).unwrap();
    // Grab the bytes of the image.
    let bytes = &buf[..info.buffer_size()];

    bytes.to_vec()
}

/// garble then eval a test .skcd
/// It is used by multiple tests to compare "specific set of inputs" vs "expected output .png"
pub fn garble_display_message_2digits(
    skcd_bytes: &[u8],
) -> (InterstellarGarbledCircuit, usize, usize) {
    let garb = garble_skcd(skcd_bytes);

    let display_config = garb.config.display_config.unwrap().clone();
    let width = display_config.width as usize;
    let height = display_config.height as usize;

    (garb, width, height)
}

/// param outputs: result of garb.eval()
/// return: the raw bytes of .png corresponding to the GarbledCircuit's eval outputs
/// Typically the is "output[i] = eval[i] * 255"
pub fn write_png(width: usize, height: usize, outputs: Vec<u16>) -> Vec<u8> {
    use std::io::BufWriter;
    use std::io::Cursor;

    // let path = "eval_outputs.png";
    let buf = Vec::new();
    let c = Cursor::new(buf);
    let ref mut w = BufWriter::new(c);

    // TODO(interstellar) get from Circuit's "config"
    let mut encoder = png::Encoder::new(w, width.try_into().unwrap(), height.try_into().unwrap());
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().unwrap();

    let data: Vec<u8> = outputs
        .iter()
        .map(|v| {
            let pixel_value: u8 = (*v).try_into().unwrap();
            pixel_value * 255
        })
        .collect();

    writer.write_image_data(&data).unwrap();

    data
}
