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

/// garble then eval a test .skcd
/// It is used by multiple tests to compare "specific set of inputs" vs "expected output .png"
pub fn garble_display_message_2digits(
    skcd_bytes: &[u8],
) -> (InterstellarGarbledCircuit, usize, usize) {
    let garb = garble_skcd(skcd_bytes).unwrap();

    let display_config = garb.config.display_config.unwrap().clone();
    let width = display_config.width as usize;
    let height = display_config.height as usize;

    (garb, width, height)
}
