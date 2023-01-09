use fancy_garbling::circuit::Circuit;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[cfg(all(not(feature = "std"), feature = "sgx"))]
use sgx_tstd::vec::Vec;

// TODO!!! add the rest of skcd.proto
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub width: u32,
    pub height: u32,
    // cf drawable::DigitSegmentsType
    // TODO!!! NOT PUB segments_type: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum GarblerInputsType {
    /// MUST be set to 0!
    Buf = 0,
    /// Part of 7 segments display; so 7 bits
    SevenSegments = 1,
    /// Part of the watermark; typically width*height nb pixels
    Watermark = 2,
}

impl TryFrom<i32> for GarblerInputsType {
    type Error = ();

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            x if x == Self::Buf as i32 => Ok(Self::Buf),
            x if x == Self::SevenSegments as i32 => Ok(Self::SevenSegments),
            x if x == Self::Watermark as i32 => Ok(Self::Watermark),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum EvaluatorInputsType {
    /// The "display circuit" standard input type: SHOULD be randomized during each eval loop
    Rnd = 0,
    /// The "generic circuit" standard input type: SHOULD be choosen by the evaluator
    /// eg for the adder circuit
    ChoosenByEvaluator = 1,
    /// Same as previous, but for the garbler
    ChoosenByGarbler = 2,
}

impl TryFrom<i32> for EvaluatorInputsType {
    type Error = ();

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            x if x == Self::Rnd as i32 => Ok(Self::Rnd),
            x if x == Self::ChoosenByEvaluator as i32 => Ok(Self::ChoosenByEvaluator),
            x if x == Self::ChoosenByGarbler as i32 => Ok(Self::ChoosenByGarbler),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct GarblerInputs {
    pub(crate) r#type: GarblerInputsType,
    pub(crate) length: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct EvaluatorInputs {
    pub(crate) r#type: EvaluatorInputsType,
    pub(crate) length: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkcdConfig {
    pub display_config: Option<DisplayConfig>,
    pub(crate) garbler_inputs: Vec<GarblerInputs>,
    pub(crate) evaluator_inputs: Vec<EvaluatorInputs>,
}

/// Represents the raw(ie UNgarbled) circuit; usually created from a .skcd file
///
/// Exists mostly to mask swanky/fancy-garbling Circuit to the public.
pub(crate) struct InterstellarCircuit {
    pub(crate) circuit: Circuit,
    pub(crate) config: SkcdConfig,
}
