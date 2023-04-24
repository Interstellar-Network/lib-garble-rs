use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::circuit::WireRef;

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

/// We need to convert something like
/// ".gate XOR  a=rnd[2] b=rnd[0] O=n7016" in the .skcd(which is basically a .blif)
/// into something that `CircuitBuilder` can accept.
/// Essentially we need to convert a String ID -> `CircuitRef`(= a usize)
///
/// IMPORTANT
/// For this to work, the INPUTS MUST also go through the same conversion, else
/// when using CircuitBuilder.or/and/etc the `CircuitRef` WOULD NOT match anything.
/// NOTE that in this case the Circuit still would build fine, but it would fail
/// when eval/garbling.
#[derive(Debug, Clone)]
pub(crate) struct SkcdToWireRefConverter {
    map_skcd_gate_id_to_circuit_ref: HashMap<String, WireRef>,
    cur_len: usize,
}

impl SkcdToWireRefConverter {
    pub fn new() -> Self {
        Self {
            map_skcd_gate_id_to_circuit_ref: HashMap::new(),
            cur_len: 0,
        }
    }

    pub fn get(&self, skcd_gate_id: &str) -> Option<&WireRef> {
        self.map_skcd_gate_id_to_circuit_ref.get(skcd_gate_id)
    }

    /// insert
    /// NOOP if already in the map
    pub fn insert(&mut self, skcd_gate_id: &str) {
        match self.get(skcd_gate_id) {
            Some(_) => {}
            None => {
                self.map_skcd_gate_id_to_circuit_ref
                    .insert(skcd_gate_id.to_string(), WireRef { id: self.cur_len });
                self.cur_len += 1;
            }
        }
    }
}
