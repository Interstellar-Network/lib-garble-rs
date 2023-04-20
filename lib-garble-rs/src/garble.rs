mod new_garbling_scheme;

use crate::circuit::InterstellarCircuit;
use crate::circuit::SkcdConfig;
use crate::garble::new_garbling_scheme::Wire;
use serde::{Deserialize, Serialize};

pub type EvaluatorInput = u16;
pub(crate) type GarblerInput = u16;

/// `EncodedGarblerInputs`: sent to the client as part of `EvaluableGarbledCircuit`
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct EncodedGarblerInputs {
    pub(crate) wires: Vec<Wire>,
}

#[derive(Debug)]
pub enum InterstellarEvaluatorError {
    EvaluatorError,
}

#[derive(Debug)]
pub enum GarblerError {
    GarblerError,
}

#[derive(PartialEq, Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "test", derive(Clone))]
pub struct GarbledCircuit {
    // TODO DO NOT Serialize the Encoder/MUST NOT be sent to the client-side b/c that probably leaks data
    // Instead we should just send the list of pair (0,1) for each EvaluatorInput only
    // pub(crate) encoder: Encoder,
    pub config: SkcdConfig,
}

impl GarbledCircuit {
    /// NOTE: it is NOT pub b/c we want to only expose the full `parse_skcd+garble`, cf lib.rs
    pub(crate) fn garble(circuit: InterstellarCircuit) -> Result<Self, GarblerError> {
        todo!()
        // .map_err(|_e| GarblerError::GarblerError)
    }

    pub(crate) fn num_evaluator_inputs(&self) -> usize {
        todo!()
    }

    pub(crate) fn num_garbler_inputs(&self) -> usize {
        todo!()
    }

    // TODO(interstellar) SHOULD NOT expose Wire; instead return a wrapper struct eg "GarblerInputs"
    pub(crate) fn encode_garbler_inputs(
        &self,
        garbler_inputs: &[GarblerInput],
    ) -> EncodedGarblerInputs {
        // TODO(interstellar)? but is this the correct time to CHECK?
        assert_eq!(
            self.num_garbler_inputs(),
            garbler_inputs.len(),
            "wrong garbler_inputs len!"
        );
        EncodedGarblerInputs { wires: todo!() }
    }

    /// Eval using Fancy-Garbling's eval(or rather `eval_with_prealloc`)
    ///
    /// # Errors
    ///
    /// `FancyError` if something went wrong during **either** eval(now)
    /// or initially when garbling!
    /// In the latter case it means the circuit is a dud and nothing can be done!
    pub fn eval(
        &mut self,
        encoded_garbler_inputs: &EncodedGarblerInputs,
        evaluator_inputs: &[EvaluatorInput],
        outputs: &mut Vec<Option<u16>>,
    ) -> Result<(), InterstellarEvaluatorError> {
        todo!()
        // let encoded_evaluator_inputs = garbled.encoder.encode_evaluator_inputs(evaluator_inputs);
        // crate::new_garble_scheme::eval(&garbled, outputs)
    }
}

#[cfg(test)]
mod tests {
    use crate::garble_skcd;
    use crate::tests::{FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS, FULL_ADDER_2BITS_ALL_INPUTS};

    /// test comparing "eval" and "eval_with_prealloc"(both to reference and b/w themselves)
    /// We only need to expose "eval_with_prealloc" publicly, but as it is a quite heavily
    /// modified version of "eval" from our own fork, it is useful to CHECK it here
    #[test]
    fn test_compare_evals_full_adder_2bits() {
        let mut garb = garble_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin")).unwrap();
        let garbler_inputs = vec![];
        let encoded_garbler_inputs = garb.encode_garbler_inputs(&garbler_inputs);

        let mut outputs_prealloc = vec![Some(0u16); FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS[0].len()];

        for (i, inputs) in FULL_ADDER_2BITS_ALL_INPUTS.iter().enumerate() {
            garb.eval(&encoded_garbler_inputs, &inputs, &mut outputs_prealloc)
                .unwrap();

            // let encoded_garbler_inputs = garb.encoder.encode_garbler_inputs(&garbler_inputs);
            // let encoded_evaluator_inputs = garb.encoder.encode_evaluator_inputs(inputs);
            // let outputs_direct = garb
            //     .garbled
            //     .eval(&encoded_garbler_inputs, &encoded_evaluator_inputs)
            //     .unwrap();

            // convert Vec<std::option::Option<u16>> -> Vec<u16>
            let outputs_prealloc: Vec<u16> = outputs_prealloc.iter().map(|i| i.unwrap()).collect();

            let expected_outputs = FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS[i];

            assert_eq!(outputs_prealloc, expected_outputs);
            // assert_eq!(outputs_direct, expected_outputs);
        }
    }
}
