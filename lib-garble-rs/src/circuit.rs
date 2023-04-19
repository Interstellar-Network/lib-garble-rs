use fancy_garbling::circuit::Circuit;
use fancy_garbling::circuit::CircuitRef;
use fancy_garbling::circuit::Gate;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TryFromPrimitive)]
#[repr(i32)]
pub(crate) enum GarblerInputsType {
    /// MUST be set to 0!
    Buf = 0,
    /// Part of 7 segments display; so 7 bits
    SevenSegments = 1,
    /// Part of the watermark; typically width*height nb pixels
    Watermark = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TryFromPrimitive)]
#[repr(i32)]
pub(crate) enum EvaluatorInputsType {
    /// The "display circuit" standard input type: SHOULD be randomized during each eval loop
    Rnd = 0,
    /// The "generic circuit" standard input type: SHOULD be choosen by the evaluator
    /// eg for the adder circuit
    ChoosenByEvaluator = 1,
    /// Same as previous, but for the garbler
    ChoosenByGarbler = 2,
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

/// Represents the raw(ie **UN**garbled) circuit; usually created from a .skcd file
///
/// Exists mostly to mask swanky/fancy-garbling Circuit to the public.
pub(crate) struct InterstellarCircuit {
    pub(crate) circuit: Circuit,
    /// This is needed for our new garbling scheme b/c we want to iterate on the gates
    /// but Fancy's Circuit DOES NOT expose the fields.
    /// This is essentially a copy of what is inside "circuit" field above.
    pub(crate) refs: Vec<CircuitRef>,
    pub(crate) gates: Vec<Gate>,
    pub(crate) config: SkcdConfig,
}
