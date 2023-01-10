use crate::circuit::InterstellarCircuit;
use crate::circuit::SkcdConfig;
use fancy_garbling::classic::{garble, Encoder, GarbledCircuit};
use fancy_garbling::errors::EvaluatorError;
use fancy_garbling::Wire;
use serde::{Deserialize, Serialize};

pub use fancy_garbling::classic::EvalCache;

#[cfg(all(not(feature = "std"), feature = "sgx"))]
use sgx_tstd::vec::Vec;

pub type EvaluatorInput = u16;
pub(crate) type GarblerInput = u16;

// TODO(interstellar) this is NOT good?? It requires the "non garbled" Circuit to be kept around
// we SHOULD (probably) rewrite "pub fn eval" in fancy-garbling/src/circuit.rs to to NOT use "self",
// and replace "circuit" by a list of ~~Gates~~/Wires?? [cf how "cache" is constructed in "fn eval"]
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct InterstellarGarbledCircuit {
    pub(crate) garbled: GarbledCircuit,
    pub(crate) encoder: Encoder,
    pub config: SkcdConfig,
}

#[cfg(test)]
impl Clone for InterstellarGarbledCircuit {
    fn clone(&self) -> InterstellarGarbledCircuit {
        InterstellarGarbledCircuit {
            garbled: self.garbled.clone(),
            encoder: self.encoder.clone(),
            config: self.config.clone(),
        }
    }
}

/// EncodedGarblerInputs: sent to the client as part of "EvaluableGarbledCircuit"
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub struct EncodedGarblerInputs {
    pub(crate) wires: Vec<Wire>,
}

#[derive(Debug)]
pub enum InterstellarEvaluatorError {
    FancyError(EvaluatorError),
}

impl InterstellarGarbledCircuit {
    /// NOTE: it is NOT pub b/c we want to only expose the full parse_skcd+garble, cf lib.rs
    pub(crate) fn garble(circuit: InterstellarCircuit) -> Self {
        let (encoder, garbled) = garble(circuit.circuit).unwrap();
        InterstellarGarbledCircuit {
            garbled,
            encoder,
            config: circuit.config,
        }
    }

    // TODO(interstellar) SHOULD NOT expose Wire; instead return a wrapper struct eg "GarblerInputs"
    pub(crate) fn encode_garbler_inputs(
        &self,
        garbler_inputs: &[GarblerInput],
    ) -> EncodedGarblerInputs {
        // TODO(interstellar)? but is this the correct time to CHECK?
        assert_eq!(
            self.encoder.num_garbler_inputs(),
            garbler_inputs.len(),
            "wrong garbler_inputs len!"
        );
        EncodedGarblerInputs {
            wires: self.encoder.encode_garbler_inputs(garbler_inputs),
        }
    }

    pub fn eval_with_prealloc(
        &mut self,
        encoded_garbler_inputs: &EncodedGarblerInputs,
        evaluator_inputs: &[EvaluatorInput],
        outputs: &mut Vec<Option<u16>>,
        eval_cache: &mut EvalCache,
    ) -> Result<(), InterstellarEvaluatorError> {
        let encoded_evaluator_inputs = self.encoder.encode_evaluator_inputs(evaluator_inputs);

        self.garbled
            .eval_with_prealloc(
                eval_cache,
                &encoded_garbler_inputs.wires,
                &encoded_evaluator_inputs,
                outputs,
            )
            .map_err(InterstellarEvaluatorError::FancyError)?;

        Ok(())
    }

    pub fn init_cache(&mut self) -> EvalCache {
        self.garbled.init_cache()
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
        let mut garb = garble_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin"));
        let garbler_inputs = vec![];
        let encoded_garbler_inputs = garb.encode_garbler_inputs(&garbler_inputs);

        let mut outputs_prealloc = vec![Some(0u16); FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS[0].len()];

        let mut eval_cache = garb.init_cache();

        for (i, inputs) in FULL_ADDER_2BITS_ALL_INPUTS.iter().enumerate() {
            garb.eval_with_prealloc(
                &encoded_garbler_inputs,
                &inputs,
                &mut outputs_prealloc,
                &mut eval_cache,
            )
            .unwrap();

            let encoded_garbler_inputs = garb.encoder.encode_garbler_inputs(&garbler_inputs);
            let encoded_evaluator_inputs = garb.encoder.encode_evaluator_inputs(inputs);
            let outputs_direct = garb
                .garbled
                .eval(&encoded_garbler_inputs, &encoded_evaluator_inputs)
                .unwrap();

            // convert Vec<std::option::Option<u16>> -> Vec<u16>
            let outputs_prealloc: Vec<u16> = outputs_prealloc.iter().map(|i| i.unwrap()).collect();

            let expected_outputs = FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS[i];

            assert_eq!(outputs_prealloc, expected_outputs);
            assert_eq!(outputs_direct, expected_outputs);
        }
    }
}
