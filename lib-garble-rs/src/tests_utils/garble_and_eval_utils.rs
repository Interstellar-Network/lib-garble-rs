use crate::garble_skcd;
use crate::EncodedGarblerInputs;
use crate::EvalCache;
use crate::EvaluatorInput;
use crate::GarbledCircuit;

use alloc::vec::Vec;
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
// TODO(opt) EncodedGarblerInputs SHOULD NOT be mut; this forces up to clone it when evaluating repeatedly
#[doc(hidden)]
#[allow(clippy::too_many_arguments, clippy::unwrap_used)]
pub fn eval_client(
    garb: &GarbledCircuit,
    encoded_garbler_inputs: &EncodedGarblerInputs,
    evaluator_inputs: &mut [EvaluatorInput],
    outputs: &mut Vec<u8>,
    eval_cache: &mut EvalCache,
    rng: &mut ThreadRng,
    rand_0_1: &Uniform<u8>,
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

    garb.eval(
        encoded_garbler_inputs,
        evaluator_inputs,
        outputs,
        eval_cache,
    )
    .unwrap();
}

/// garble then eval a test .skcd
/// It is used by multiple tests to compare "specific set of inputs" vs "expected output .png"
#[doc(hidden)]
#[allow(clippy::unwrap_used, clippy::must_use_candidate)]
pub fn garble_skcd_helper(skcd_bytes: &[u8]) -> (GarbledCircuit, usize, usize) {
    let garb = garble_skcd(skcd_bytes).unwrap();

    let display_config = garb.config.display_config.unwrap();
    let width = display_config.width as usize;
    let height = display_config.height as usize;

    (garb, width, height)
}
