// #![no_std]
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(elided_lifetimes_in_paths)]

#[cfg(all(not(feature = "std"), feature = "sgx"))]
extern crate sgx_tstd as std;

#[cfg(all(not(feature = "std"), feature = "sgx"))]
use sgx_tstd::vec;

extern crate alloc;

mod circuit;
mod garble;
mod serialize_deserialize;
mod skcd_parser;
mod watermark;

// re-export
pub use garble::EncodedGarblerInputs;
pub use garble::EvalCache;
pub use garble::EvaluatorInput;
pub use garble::InterstellarGarbledCircuit;
pub use serialize_deserialize::{deserialize_for_evaluator, serialize_for_evaluator};

/// This is the main entry point of this function; meant to be called by the "pallet-ocw-garble"
///
/// It:
/// - parses a .skcd; usually coming from IPFS
/// - garbles it
/// - encode the "garbler inputs" ie the message/watermark/OTP(pinpad or message)
// TODO it SHOULD return a serialized GC, with "encoded inputs"
pub fn garble_skcd(skcd_buf: &[u8]) -> InterstellarGarbledCircuit {
    let circ = circuit::InterstellarCircuit::parse_skcd(skcd_buf).unwrap();

    InterstellarGarbledCircuit::garble(circ)
}

/// Prepare the garbler_inputs; it contains both:
/// - the watermark(ie the message)
/// - the 7 segments digits
/// NOTE: this is ONLY applicable to "display circuits"
///
// TODO(interstellar) randomize 7 segs(then replace "garbler_input_segments")
// TODO(interstellar) the number of digits DEPENDS on the config!
pub fn garbled_display_circuit_prepare_garbler_inputs(
    garb: &InterstellarGarbledCircuit,
    watermark_text: &str,
) -> EncodedGarblerInputs {
    let watermark_font = watermark::new_font();
    let watermark = watermark::draw_text(
        garb.config
            .display_config
            .expect("no display_config! circuit is not a display circuit?")
            .width,
        garb.config
            .display_config
            .expect("no display_config! circuit is not a display circuit?")
            .height,
        &watermark_font,
        watermark_text,
    );

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
        garbler_input_buf,
        garbler_input_segments,
        garbler_input_watermark,
    ]
    .concat();

    garb.encode_garbler_inputs(&garbler_inputs)
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::garble_skcd;

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
        let encoded_garbler_inputs = garb.encode_garbler_inputs(&[]);

        let mut outputs = vec![Some(0u16); FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS[0].len()];

        let mut eval_cache = garb.init_cache();

        for (i, inputs) in FULL_ADDER_2BITS_ALL_INPUTS.iter().enumerate() {
            garb.eval_with_prealloc(
                &encoded_garbler_inputs,
                &inputs,
                &mut outputs,
                &mut eval_cache,
            )
            .unwrap();

            // convert Vec<std::option::Option<u16>> -> Vec<u16>
            let outputs: Vec<u16> = outputs.iter().map(|i| i.unwrap()).collect();

            let expected_outputs = FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS[i];
            println!(
                "inputs = {:?}, outputs = {:?}, expected_outputs = {:?}",
                inputs, outputs, expected_outputs
            );
            assert_eq!(outputs, expected_outputs);
        }
    }

    // NOTE: more tests with "display circuits" are in tests/ folder
}
