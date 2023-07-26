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
use alloc::vec;
use alloc::vec::Vec;
use snafu::prelude::*;

use circuit_types_rs::{EvaluatorInputsType, GarblerInputsType};

// re-export
pub use garble::{EncodedGarblerInputs, EvaluatorInput, GarbledCircuit};
pub use new_garbling_scheme::evaluate::EvalCache;
pub use serialize_deserialize::{deserialize_for_evaluator, serialize_for_evaluator};

mod garble;
mod new_garbling_scheme;
mod segments;
mod serialize_deserialize;
mod watermark;

#[derive(Debug, Snafu, PartialEq)]
pub enum InterstellarError {
    /// Error at GarbledCircuit::garble
    GarblerError,
    /// Error at garbled_display_circuit_prepare_garbler_inputs
    SkcdParserError,
    /// garbled_display_circuit_prepare_garbler_inputs: the circuit SHOULD be
    /// a "display circuit"; ie it MUST contain a valid config with field "display_config" set
    NotAValidDisplayCircuit,
    OnlyValidForDisplayCircuit,
    /// The given integer is NOT a valid 7 segments option[ie 0-9]
    NotAValid7Segment {
        digit: u8,
    },
    /// "BUF garbler_input SHOULD be of length == 1"
    GarblerInputsInvalidBufLength,
    /// SevenSegments garbler_input SHOULD be of length % 7
    GarblerInputs7SegmentsNotMod7,
    /// SevenSegments garbler_input SHOULD match digits parameter
    GarblerInputs7SegmentsWrongLength,
    /// error during `new_watermark`
    WatermarkError {
        msg: String,
    },
    SerializerDeserializerInternalError {
        err: postcard::Error,
    },
    /// "wrong encoded_garbler_inputs len!"
    SerializeForEvaluatorWrongInputsLength {
        inputs_len: usize,
        expected_len: usize,
    },
}

#[derive(Debug)]
pub enum InterstellarEvaluatorError {
    /// Error at `decoding_internal`
    DecodingErrorMissingOutputLabel {
        idx: usize,
    },
    /// Error at `evaluate_internal`
    EvaluateErrorMissingLabel {
        idx: usize,
    },
    /// Error at `evaluate_internal`
    EvaluateErrorMissingDelta {
        idx: usize,
    },
    BaseError {
        err: InterstellarError,
    },
}

impl From<InterstellarError> for InterstellarEvaluatorError {
    fn from(value: InterstellarError) -> Self {
        Self::BaseError { err: value }
    }
}

/// This is the main entry point of this function; meant to be called by the "pallet-ocw-garble"
///
/// It:
/// - parses a .skcd; usually coming from IPFS
/// - garbles it
///
/// # Errors
/// - if the circuit can not be parsed; eg `skcd_buf` does not contain properly serialized data(postcard)
/// - something went wrong during `garble`
///
// TODO it SHOULD return a serialized GC, with "encoded inputs"
pub fn garble_skcd(skcd_buf: &[u8]) -> Result<GarbledCircuit, InterstellarError> {
    garble_skcd_aux(skcd_buf, None)
}

fn garble_skcd_aux(
    skcd_buf: &[u8],
    rng_seed: Option<u64>,
) -> Result<GarbledCircuit, InterstellarError> {
    let circuit = circuit_types_rs::deserialize_from_buffer(skcd_buf)
        .map_err(|_e| InterstellarError::SkcdParserError)?;

    let garbled = new_garbling_scheme::garble::garble(circuit, rng_seed)
        .map_err(|_e| InterstellarError::GarblerError)?;

    Ok(GarbledCircuit::new(garbled))
}

/// Variant of `garble_skcd` used for tests
///
/// # Arguments
///
/// * `rng_seed` - when None; it will use the standard and secure `ChaChaRng::from_entropy`
///     when given: it will use the NOT SECURE `seed_from_u64`
///
/// # Errors
/// cf `garble_skcd`
///
pub fn garble_skcd_with_seed(
    skcd_buf: &[u8],
    rng_seed: u64,
) -> Result<GarbledCircuit, InterstellarError> {
    garble_skcd_aux(skcd_buf, Some(rng_seed))
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
    garb: &GarbledCircuit,
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
    let display_config = garb.get_display_config()?;
    let mut garbler_inputs = Vec::with_capacity(
        display_config
            .garbler_inputs
            .iter()
            .fold(0, |acc, e| acc + e.length as usize),
    );
    for garbler_input in &display_config.garbler_inputs {
        match garbler_input.r#type {
            GarblerInputsType::Buf => {
                if garbler_input.length != 1 {
                    return Err(InterstellarError::GarblerInputsInvalidBufLength);
                }

                garbler_inputs.push(0u8);
            }
            GarblerInputsType::SevenSegments => {
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
            GarblerInputsType::Watermark => {
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

    Ok(garb.encode_inputs(&garbler_inputs))
}

/// Like `garbled_display_circuit_prepare_garbler_inputs` but for the client-side(ie Evaluator)
/// Initialize a Vec for the "to be randomized each eval loop" evaluator inputs
///
/// # Errors
///
/// # Panics
///
/// TODO! If the given circuit if NOT a "display circuit" it will panic instead of properly passing to the client
pub fn prepare_evaluator_inputs(
    garb: &GarbledCircuit,
) -> Result<Vec<EvaluatorInput>, InterstellarError> {
    let display_config = garb.get_display_config()?;
    let mut evaluator_inputs = Vec::with_capacity(
        display_config
            .evaluator_inputs
            .iter()
            .fold(0, |acc, e| acc + e.length as usize),
    );

    for evaluator_input in &display_config.evaluator_inputs {
        match evaluator_input.r#type {
            EvaluatorInputsType::Rnd => {
                let mut inputs_0 = vec![0; evaluator_input.length as usize];
                evaluator_inputs.append(&mut inputs_0);
            }
        }
    }

    Ok(evaluator_inputs)
}

#[doc(hidden)]
#[cfg(feature = "std")]
pub mod tests_utils;

#[cfg(test)]
mod tests {

    use super::*;

    // all_inputs/all_expected_outputs: standard full-adder 2 bits truth table(and expected results)
    // input  i_bit1;
    // input  i_bit2;
    // input  i_carry;
    pub(super) const FULL_ADDER_2BITS_ALL_INPUTS: [[u8; 3]; 8] = [
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
    pub(super) const FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS: [[u8; 2]; 8] = [
        [0, 0],
        [1, 0],
        [1, 0],
        [0, 1],
        [1, 0],
        [0, 1],
        [0, 1],
        [1, 1],
    ];

    #[test]
    fn test_garble_evaluate_full_adder_2bits() {
        let garb = garble_skcd(include_bytes!(
            "../examples/data/result_abc_full_adder.postcard.bin"
        ))
        .unwrap();
        let encoded_garbler_inputs = garb.encode_inputs(&[]);

        let mut outputs = vec![0u8; FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS[0].len()];
        let mut eval_cache = EvalCache::new();

        for test_idx in 0..10 {
            for (i, inputs) in FULL_ADDER_2BITS_ALL_INPUTS.iter().enumerate() {
                garb.eval(
                    &encoded_garbler_inputs,
                    inputs,
                    &mut outputs,
                    &mut eval_cache,
                )
                .unwrap();

                let expected_outputs = FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS[i];
                assert_eq!(
                    outputs, expected_outputs,
                    "inputs = {inputs:?}, outputs = {outputs:?}, expected_outputs = {expected_outputs:?}, at test nb [{test_idx},{i}]"
                );
            }
        }
    }

    // NOTE: more tests with "display circuits" are in tests/ folder
}
