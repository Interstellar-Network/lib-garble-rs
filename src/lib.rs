// #![no_std]
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(elided_lifetimes_in_paths)]

#[cfg(all(not(feature = "std"), feature = "sgx"))]
extern crate sgx_tstd as std;

extern crate alloc;

mod circuit;
mod garble;
mod skcd_parser;
// TODO(interstellar) put behind a feature; the client DOES NOT need it
pub mod ipfs;
pub mod watermark;

/// This is the main entry point of this function; meant to be called by the "pallet-ocw-garble"
///
/// It:
/// - parses a .skcd; usually coming from IPFS
/// - garbles it
/// - encode the "garbler inputs" ie the message/watermark/OTP(pinpad or message)
// TODO it SHOULD return a serialized GC, with "encoded inputs"
pub fn garble_skcd(skcd_buf: &[u8]) -> garble::InterstellarGarbledCircuit {
    let circ = circuit::InterstellarCircuit::parse_skcd(skcd_buf).unwrap();

    let mut garb = garble::InterstellarGarbledCircuit::garble(circ);

    garb
}

#[cfg(test)]
pub(crate) mod tests {
    use super::garble_skcd;
    use crate::circuit::InterstellarCircuit;
    use crate::garble::EvaluatorInput;
    use crate::garble::InterstellarGarbledCircuit;
    use fancy_garbling::Wire;

    // all_inputs/all_expected_outputs: standard full-adder 2 bits truth table(and expected results)
    // input  i_bit1;
    // input  i_bit2;
    // input  i_carry;
    pub(crate) const FULL_ADDER_2BITS_ALL_INPUTS: &'static [&'static [u16]] = &[
        &[0, 0, 0],
        &[1, 0, 0],
        &[0, 1, 0],
        &[1, 1, 0],
        &[0, 0, 1],
        &[1, 0, 1],
        &[0, 1, 1],
        &[1, 1, 1],
    ];

    // output o_sum;
    // output o_carry;
    pub(crate) const FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS: &'static [&'static [u16]] = &[
        &[0, 0],
        &[1, 0],
        &[1, 0],
        &[0, 1],
        &[1, 0],
        &[0, 1],
        &[0, 1],
        &[1, 1],
    ];

    #[test]
    fn test_garble_full_adder_2bits() {
        let mut garb = garble_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin"));

        for (i, inputs) in FULL_ADDER_2BITS_ALL_INPUTS.iter().enumerate() {
            let outputs = garb.eval(&[], inputs).unwrap();
            let expected_outputs = FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS[i];
            println!(
                "inputs = {:?}, outputs = {:?}, expected_outputs = {:?}",
                inputs, outputs, expected_outputs
            );
            assert_eq!(outputs, expected_outputs);
        }
    }

    /// cf https://docs.rs/png/latest/png/#using-the-decoder
    fn read_png_to_bytes(buf: &[u8]) -> Vec<u8> {
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
    fn garble_display_message_2digits(
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
    fn write_png(width: usize, height: usize, outputs: Vec<u16>) -> Vec<u8> {
        use std::io::BufWriter;
        use std::io::Cursor;

        // let path = "eval_outputs.png";
        let buf = Vec::new();
        let c = Cursor::new(buf);
        let ref mut w = BufWriter::new(c);

        // TODO(interstellar) get from Circuit's "config"
        let mut encoder =
            png::Encoder::new(w, width.try_into().unwrap(), height.try_into().unwrap());
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

    // TODO!!! MUST combine multiple evals; or alternatively have several tests with different "evaluator_inputs"
    #[test]
    fn test_garble_display_message_120x52_2digits_ones() {
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
        let garbler_input_watermark = vec![0u16; 120 * 52];

        let garbler_inputs = [
            garbler_input_buf.clone(),
            garbler_input_segments.clone(),
            garbler_input_watermark.clone(),
        ]
        .concat();

        let (mut garb, width, height) = garble_display_message_2digits(include_bytes!(
            "../examples/data/display_message_120x52_2digits.skcd.pb.bin"
        ));
        let data = garb
            .eval(&garbler_inputs, &[0u16, 1, 0, 1, 0, 1, 0, 1, 0])
            .unwrap();
        let eval_outputs = write_png(width, height, data);

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
        let data = garb.eval(&[0; 1 + 2 * 7 + 120 * 52], &[0; 9]).unwrap();
        let eval_outputs = write_png(width, height, data);

        let expected_outputs = read_png_to_bytes(include_bytes!(
            "../examples/data/eval_outputs_display_message_120x52_2digits_inputs0.png"
        ));
        assert_eq!(eval_outputs, expected_outputs);
    }

    /// Client use-case, or as close as possible.
    /// NOT using the "standard API" b/c that re-encodes teh garbler_inputs every eval
    /// That costs around ~5ms...
    /// let data = garb.eval(&garbler_inputs, &[0; 9]).unwrap();
    // #[profiling::function]
    fn eval_client(
        garb: &mut InterstellarGarbledCircuit,
        garbler_inputs: &Vec<Wire>,
        evaluator_inputs: &[EvaluatorInput],
        data: &mut Vec<Option<u16>>,
    ) {
        // coz::scope!("eval_client");

        let evaluator_inputs = &garb.encoder.encode_evaluator_inputs(evaluator_inputs);
        garb.garbled
            .eval_with_prealloc(&garbler_inputs, &evaluator_inputs, data)
            .unwrap();
    }

    // NOTE it is quite slow in Debug! Make sure to enable optimizations
    #[test]
    fn bench_eval_display_message_640x360_2digits_42() {
        use rand::distributions::Uniform;
        use rand::prelude::Distribution;
        use rand::thread_rng;
        use std::time::Instant;

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
        let garbler_input_watermark = vec![0u16; 640 * 360];

        let garbler_inputs = [
            garbler_input_buf.clone(),
            garbler_input_segments.clone(),
            garbler_input_watermark.clone(),
        ]
        .concat();

        let mut eval_times = Vec::with_capacity(NB_EVALS);
        // To try and make sure eval is run and NOT optimized-out
        let mut eval_datas = Vec::with_capacity(NB_EVALS);

        let (mut garb, width, height) = garble_display_message_2digits(include_bytes!(
            "../examples/data/display_message_640x360_2digits.skcd.pb.bin"
        ));

        let garbler_inputs = garb.encoder.encode_garbler_inputs(&garbler_inputs);

        let mut rng = thread_rng();
        let rand_0_1 = Uniform::from(0..=1);

        let mut evaluator_inputs = vec![
            // "rnd": 9 inputs
            0u16, 0, 0, 0, 0, 0, 0, 0, 0, //
        ];

        let mut data = vec![Some(0u16); width * height];
        garb.init_cache();

        for _ in 0..NB_EVALS {
            // profiling::scope!("Looped eval");
            // coz::progress!();

            let start = Instant::now();

            // randomize the "rnd" part of the inputs
            // cf "rndswitch.v" comment above; DO NOT touch the last!
            for input in evaluator_inputs.iter_mut() {
                *input = rand_0_1.sample(&mut rng);
            }

            eval_client(&mut garb, &garbler_inputs, &evaluator_inputs, &mut data);

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
}
