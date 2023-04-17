use fancy_garbling::circuit::Circuit;
use serde::{Deserialize, Serialize};

use crate::{
    garble::{GarblerError, InterstellarEvaluatorError},
    InterstellarGarbledCircuit,
};

#[derive(PartialEq, Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "test", derive(Clone))]
pub(crate) struct GarbledCircuit {}

pub(crate) fn garble(circuit: Circuit) -> Result<InterstellarGarbledCircuit, GarblerError> {
    Ok(InterstellarGarbledCircuit {
        garbled: GarbledCircuit {},
        encoder: todo!(),
        config: todo!(),
    })
}

pub(crate) fn eval(
    garbled: &GarbledCircuit,
    outputs: &mut Vec<Option<u16>>,
) -> Result<(), InterstellarEvaluatorError> {
    Ok(())
}
