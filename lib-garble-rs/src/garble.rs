// mod new_garbling_scheme;

// use crate::circuit::Circuit;
// use crate::circuit::SkcdConfig;
// use serde::{Deserialize, Serialize};

// use self::new_garbling_scheme::garble::GarbledCircuitFinal;

// pub type EvaluatorInput = u8;
// pub(super) type GarblerInput = u8;

// #[derive(Debug)]
// pub enum InterstellarEvaluatorError {
//     EvaluatorError,
// }

// #[derive(PartialEq, Debug, Deserialize, Serialize)]
// #[cfg_attr(feature = "test", derive(Clone))]
// pub struct GarbledCircuit {
//     // TODO DO NOT Serialize the `GarbleCircuit`[at least not entirely]
//     // MUST NOT be sent to the client-side b/c that probably leaks data
//     // Instead we should just send the list of pair (0,1) for each EvaluatorInput only
//     pub(super) garbled: GarbledCircuitFinal,
//     pub config: SkcdConfig,
// }

// impl GarbledCircuit {
//     /// NOTE: it is NOT pub b/c we want to only expose the full `parse_skcd+garble`, cf lib.rs
//     pub(super) fn garble(circuit: Circuit) -> Result<Self, GarblerError> {
//         let garbled = new_garbling_scheme::garble::garble(circuit.circuit)?;
//         Ok(Self {
//             garbled,
//             config: circuit.config,
//         })
//     }

//     // TODO(interstellar) SHOULD NOT expose Wire; instead return a wrapper struct eg "GarblerInputs"
//     pub(super) fn encode_garbler_inputs(
//         &self,
//         garbler_inputs: &[GarblerInput],
//     ) -> EncodedGarblerInputs {
//         // TODO(interstellar)? but is this the correct time to CHECK?
//         assert_eq!(
//             self.num_garbler_inputs() as usize,
//             garbler_inputs.len(),
//             "wrong garbler_inputs len!"
//         );
//         EncodedGarblerInputs {
//             encoded_wires: new_garbling_scheme::evaluate::evaluate(garbled, x),
//         }
//     }

//     /// Eval using Fancy-Garbling's eval(or rather `eval_with_prealloc`)
//     ///
//     /// # Errors
//     ///
//     /// `FancyError` if something went wrong during **either** eval(now)
//     /// or initially when garbling!
//     /// In the latter case it means the circuit is a dud and nothing can be done!
//     pub fn eval(
//         &mut self,
//         encoded_garbler_inputs: &EncodedGarblerInputs,
//         evaluator_inputs: &[EvaluatorInput],
//         outputs: &mut Vec<u8>,
//     ) -> Result<(), InterstellarEvaluatorError> {
//         todo!()
//         // let encoded_evaluator_inputs = garbled.encoder.encode_evaluator_inputs(evaluator_inputs);
//         // crate::new_garble_scheme::eval(&garbled, outputs)
//     }
// }
