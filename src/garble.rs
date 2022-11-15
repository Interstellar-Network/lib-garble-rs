use crate::circuit::InterstellarCircuit;
use fancy_garbling::circuit::{Circuit, CircuitRef, Gate};
use fancy_garbling::classic::{garble, Encoder, GarbledCircuit};
use fancy_garbling::errors::{EvaluatorError, FancyError};
use fancy_garbling::Fancy;

// TODO(interstellar) this is NOT good?? It requires the "non garbled" Circuit to be kept around
// we SHOULD (probably) rewrite "pub fn eval" in fancy-garbling/src/circuit.rs to to NOT use "self",
// and replace "circuit" by a list of ~~Gates~~/Wires?? [cf how "cache" is constructed in "fn eval"]
pub struct InterstellarGarbledCircuit {
    garbled: GarbledCircuit,
    encoder: Encoder,
    circuit: InterstellarCircuit,
}

#[derive(Debug)]
pub enum InterstellarEvaluatorError {
    FancyError(EvaluatorError),
}

impl InterstellarGarbledCircuit {
    pub fn garble(circuit: InterstellarCircuit) -> Self {
        let (encoder, garbled) = garble(&circuit.circuit).unwrap();
        InterstellarGarbledCircuit {
            garbled: garbled,
            encoder: encoder,
            circuit: circuit,
        }
    }

    pub fn eval(
        &self,
        evaluator_inputs: &[u16],
        garbler_inputs: &[u16],
    ) -> Result<Vec<u16>, InterstellarEvaluatorError> {
        let evaluator_inputs = &self.encoder.encode_evaluator_inputs(&evaluator_inputs);
        let garbler_inputs = &self.encoder.encode_garbler_inputs(&garbler_inputs);

        self.garbled
            .eval(&self.circuit.circuit, garbler_inputs, evaluator_inputs)
            .map_err(|e| InterstellarEvaluatorError::FancyError(e))
    }
}

/// "Evaluate the circuit using fancy object `f`."
///
/// Copy-pasted from fancy-garbling/src/circuit.rs "fn eval"
/// but modified to take a Circuit as parameter instead of "self"
/// and split into two:
/// - prepare the "cache": convert the Circuit into a Vec<Wire>
///   This is done server-side, at garbling time.
/// - use the cache for the "eval"
///   This is the client-side part.
// TODO(interstellar) remove garbler_inputs/evaluator_inputs from here and move to "second part"
//  How to handle them? Let those at None, and then?
pub fn fancy_eval_prepare<F: Fancy>(
    circuit: &Circuit,
    f: &mut F,
    garbler_inputs: &[F::Item],
    evaluator_inputs: &[F::Item],
) -> Result<Vec<Option<F::Item>>, F::Error> {
    let mut cache: Vec<Option<F::Item>> = vec![None; circuit.gates.len()];
    for (i, gate) in circuit.gates.iter().enumerate() {
        let q = circuit.modulus(i);
        let (zref_, val) = match *gate {
            Gate::GarblerInput { id } => (None, garbler_inputs[id].clone()),
            Gate::EvaluatorInput { id } => {
                assert!(
                    id < evaluator_inputs.len(),
                    "id={} ev_inps.len()={}",
                    id,
                    evaluator_inputs.len()
                );
                (None, evaluator_inputs[id].clone())
            }
            Gate::Constant { val } => (None, f.constant(val, q)?),
            Gate::Add { xref, yref, out } => (
                out,
                f.add(
                    cache[xref.ix]
                        .as_ref()
                        .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                    cache[yref.ix]
                        .as_ref()
                        .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                )?,
            ),
            Gate::Sub { xref, yref, out } => (
                out,
                f.sub(
                    cache[xref.ix]
                        .as_ref()
                        .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                    cache[yref.ix]
                        .as_ref()
                        .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                )?,
            ),
            Gate::Cmul { xref, c, out } => (
                out,
                f.cmul(
                    cache[xref.ix]
                        .as_ref()
                        .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                    c,
                )?,
            ),
            Gate::Proj {
                xref, ref tt, out, ..
            } => (
                out,
                f.proj(
                    cache[xref.ix]
                        .as_ref()
                        .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                    q,
                    Some(tt.to_vec()),
                )?,
            ),
            Gate::Mul {
                xref, yref, out, ..
            } => (
                out,
                f.mul(
                    cache[xref.ix]
                        .as_ref()
                        .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                    cache[yref.ix]
                        .as_ref()
                        .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?,
                )?,
            ),
        };
        cache[zref_.unwrap_or(i)] = Some(val);
    }

    Ok(cache)
}

// /// param cache: result from "fn fancy_eval_prepare" above
// /// param output_refs: fancy_garbling::circuit::Circuit.output_refs
// fn fancy_eval<F: Fancy>(
//     cache: Vec<Option<F::Item>>,
//     output_refs: Vec<CircuitRef>,
// ) -> Result<Option<Vec<u16>>, F::Error> {
//     let mut outputs = Vec::with_capacity(output_refs.len());
//     for r in output_refs.iter() {
//         let r = cache[r.ix]
//             .as_ref()
//             .ok_or_else(|| F::Error::from(FancyError::UninitializedValue))?;
//         let out = f.output(r)?;
//         outputs.push(out);
//     }
//     Ok(outputs.into_iter().collect())
// }
