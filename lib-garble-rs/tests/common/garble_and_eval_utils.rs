use lib_garble_rs::garble_skcd;
use lib_garble_rs::EncodedGarblerInputs;
use lib_garble_rs::EvaluatorInput;
use lib_garble_rs::GarbledCircuit;

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
    garb: &mut GarbledCircuit,
    encoded_garbler_inputs: &mut EncodedGarblerInputs,
    evaluator_inputs: &mut [EvaluatorInput],
    data: &mut Vec<u8>,
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

    garb.eval(encoded_garbler_inputs, evaluator_inputs, data)
        .unwrap();
}

/// garble then eval a test .skcd
/// It is used by multiple tests to compare "specific set of inputs" vs "expected output .png"
pub fn garble_skcd_helper(skcd_bytes: &[u8]) -> (GarbledCircuit, usize, usize) {
    let garb = garble_skcd(skcd_bytes).unwrap();

    let display_config = garb.config.display_config.unwrap().clone();
    let width = display_config.width as usize;
    let height = display_config.height as usize;

    (garb, width, height)
}
