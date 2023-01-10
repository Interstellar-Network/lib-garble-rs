// #![no_std]
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(elided_lifetimes_in_paths)]
#![warn(clippy::suspicious)]
#![warn(clippy::complexity)]
#![warn(clippy::perf)]
#![warn(clippy::style)]
#![warn(clippy::pedantic)]
#![warn(clippy::expect_used)]
#![warn(clippy::panic)]
#![warn(clippy::unwrap_used)]

extern crate alloc;

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use snafu::prelude::*;

#[cfg(all(not(feature = "std"), feature = "sgx"))]
extern crate sgx_tstd as std;

#[cfg(all(not(feature = "std"), feature = "sgx"))]
use sgx_tstd::vec;

mod circuit;
mod garble;
mod segments;
mod serialize_deserialize;
mod skcd_parser;
mod watermark;

// re-export
pub use garble::EncodedGarblerInputs;
pub use garble::EvalCache;
pub use garble::EvaluatorInput;
pub use garble::InterstellarGarbledCircuit;
pub use serialize_deserialize::{deserialize_for_evaluator, serialize_for_evaluator};

#[derive(Debug, Snafu)]
pub enum InterstellarError {
    /// Error at InterstellarGarbledCircuit::garble
    GarbleError,
    /// Error at garbled_display_circuit_prepare_garbler_inputs
    SkcdParserError,
    /// garbled_display_circuit_prepare_garbler_inputs: the circuit SHOULD be
    /// a "display circuit"; ie it MUST contain a valid config with field "display_config" set
    NotAValidDisplayCircuit,
    /// The given integer is NOT a valid 7 segments option[ie 0-9]
    NotAValid7Segment { digit: u8 },
    /// "BUF garbler_input SHOULD be of length == 1"
    GarblerInputsInvalidBufLength,
    /// SevenSegments garbler_input SHOULD be of length % 7
    GarblerInputs7SegmentsNotMod7,
    /// SevenSegments garbler_input SHOULD match digits parameter
    GarblerInputs7SegmentsWrongLength,
    /// error during `new_watermark`
    WatermarkError { msg: String },
}

/// This is the main entry point of this function; meant to be called by the "pallet-ocw-garble"
///
/// It:
/// - parses a .skcd; usually coming from IPFS
/// - garbles it
/// - encode the "garbler inputs" ie the message/watermark/OTP(pinpad or message)
///
/// # Errors
/// - if the circuit can not be parsed; eg `skcd_buf` does not contain properly serialized data(postcard)
/// - something went wrong during `garble`
///
// TODO it SHOULD return a serialized GC, with "encoded inputs"
pub fn garble_skcd(skcd_buf: &[u8]) -> Result<InterstellarGarbledCircuit, InterstellarError> {
    let circ = circuit::InterstellarCircuit::parse_skcd(skcd_buf)
        .map_err(|_e| InterstellarError::SkcdParserError)?;

    InterstellarGarbledCircuit::garble(circ).map_err(|_e| InterstellarError::GarbleError)
}

/// Prepare the `garbler_inputs`; it contains both:
/// - the watermark(ie the message)
/// - the 7 segments digits
/// NOTE: this is ONLY applicable to "display circuits"
///
/// # Errors
///
/// Will return en error when:
/// - "digits" contains value outside the valid 7 segments range [0-9]
/// - the inputs(ie "digits") length do not match what the circuit "garb" expects
///   eg if "garb" expects 14 bits of `garbler_input` for  7 segments -> digits.len() == 2
// TODO(interstellar) randomize 7 segs(then replace "garbler_input_segments")
// TODO(interstellar) the number of digits DEPENDS on the config!
pub fn garbled_display_circuit_prepare_garbler_inputs(
    garb: &InterstellarGarbledCircuit,
    digits: &[u8],
    watermark_text: &str,
) -> Result<EncodedGarblerInputs, InterstellarError> {
    // Those are splitted into:
    // - "buf" gate (cf Verilog "rndswitch.v"; and correspondingly lib_garble/src/packmsg/packmsg_utils.cpp PrepareInputLabels);
    //    it MUST always be 0 else the 7 segments will not work as expected = 1 bit
    // - the segments to display: 7 segments * "nb of digits in the message" = 7 * N bits
    // - the watermark; one bit per pixel in the final display = width * height bits
    //
    // prepare using the correct garbler_inputs total length(in BITS)
    // ie simply sum the length of each GarblerInput
    let mut garbler_inputs = Vec::with_capacity(
        garb.config
            .garbler_inputs
            .iter()
            .fold(0, |acc, e| acc + e.length as usize),
    );
    for garbler_input in &garb.config.garbler_inputs {
        match garbler_input.r#type {
            circuit::GarblerInputsType::Buf => {
                if garbler_input.length != 1 {
                    return Err(InterstellarError::GarblerInputsInvalidBufLength);
                }

                garbler_inputs.push(0u16);
            }
            circuit::GarblerInputsType::SevenSegments => {
                if garbler_input.length % 7 != 0 {
                    return Err(InterstellarError::GarblerInputs7SegmentsNotMod7);
                }
                if garbler_input.length as usize != digits.len() * 7 {
                    return Err(InterstellarError::GarblerInputs7SegmentsWrongLength);
                }

                let mut segments_inputs = segments::digits_to_segments_bits(digits)
                    .map_err(|e| InterstellarError::NotAValid7Segment { digit: e.number })?;
                garbler_inputs.append(&mut segments_inputs);
            }
            circuit::GarblerInputsType::Watermark => {
                let display_config = garb
                    .config
                    .display_config
                    .ok_or(InterstellarError::NotAValidDisplayCircuit)?;
                let mut watermark_inputs = watermark::new_watermark(
                    display_config.width,
                    display_config.height,
                    watermark_text,
                )
                .map_err(|err| InterstellarError::WatermarkError {
                    msg: err.to_string(),
                })?;
                garbler_inputs.append(&mut watermark_inputs);
            }
        }
    }

    Ok(garb.encode_garbler_inputs(&garbler_inputs))
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
        let mut garb = garble_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin")).unwrap();
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
