pub mod circuit;
pub mod garble;
mod skcd_parser;

#[cfg(test)]
mod tests {
    use crate::circuit::InterstellarCircuit;

    // all_inputs/all_expected_outputs: standard full-adder 2 bits truth table(and expected results)
    // input  i_bit1;
    // input  i_bit2;
    // input  i_carry;
    const FULL_ADDER_2BITS_ALL_INPUTS: &'static [&'static [u16]] = &[
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
    const FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS: &'static [&'static [u16]] = &[
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
    fn test_eval_plain_full_adder_2bits() {
        let circ =
            InterstellarCircuit::parse_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin"))
                .unwrap();

        assert!(circ.num_evaluator_inputs() == 3);
        for (i, inputs) in FULL_ADDER_2BITS_ALL_INPUTS.iter().enumerate() {
            let outputs = circ.eval_plain(&[], inputs).unwrap();
            assert_eq!(outputs, FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS[i]);
        }
    }

    #[test]
    fn test_garble_full_adder_2bits() {
        use crate::garble::InterstellarGarbledCircuit;

        let circ =
            InterstellarCircuit::parse_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin"))
                .unwrap();

        let mut garb = InterstellarGarbledCircuit::garble(circ);

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
        garbler_inputs: &[u16],
        evaluator_inputs: &[u16],
    ) -> Vec<u8> {
        use crate::garble::InterstellarGarbledCircuit;
        use std::io::BufWriter;
        use std::io::Cursor;

        let circ = InterstellarCircuit::parse_skcd(skcd_bytes).unwrap();

        let display_config = circ.config.display_config.unwrap().clone();
        let width = display_config.width as usize;
        let height = display_config.height as usize;

        let mut garb = InterstellarGarbledCircuit::garble(circ);

        let outputs = garb.eval(garbler_inputs, evaluator_inputs).unwrap();

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

        let data = garble_display_message_2digits(
            include_bytes!("../examples/data/display_message_120x52_2digits.skcd.pb.bin"),
            &garbler_inputs,
            &[0u16, 1, 0, 1, 0, 1, 0, 1, 0],
        );

        let expected_outputs = read_png_to_bytes(include_bytes!(
            "../examples/data/eval_outputs_display_message_120x52_2digits_42.png"
        ));
        assert_eq!(data, expected_outputs);
    }

    #[test]
    fn test_garble_display_message_120x52_2digits_zeros() {
        let data = garble_display_message_2digits(
            include_bytes!("../examples/data/display_message_120x52_2digits.skcd.pb.bin"),
            &[0; 1 + 2 * 7 + 120 * 52],
            &[0; 9],
        );

        let expected_outputs = read_png_to_bytes(include_bytes!(
            "../examples/data/eval_outputs_display_message_120x52_2digits_inputs0.png"
        ));
        assert_eq!(data, expected_outputs);
    }

    // TODO!!! but it is quite slow? maybe just add a "mark"
    // #[test]
    fn test_garble_display_message_640x360_2digits_42() {
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

        let data = garble_display_message_2digits(
            include_bytes!("../examples/data/display_message_640x360_2digits.skcd.pb.bin"),
            &garbler_inputs,
            &[0; 9],
        );

        let expected_outputs = read_png_to_bytes(include_bytes!(
            "../examples/data/eval_outputs_display_message_640x360_2digits_42.png"
        ));
        assert_eq!(data, expected_outputs);
    }
}
