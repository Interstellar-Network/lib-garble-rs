//! NOTE: everything in this module is here to avoid serializing `GateType` and sending it all the way
//! to the evaluators...
//! We could alternatively simply "embed" `Circuit` into `GarbledCircuit` and not care about this.
//!

use circuit_types_rs::{Circuit, DisplayConfig, Gate, GateType, Metadata, WireRef};
use serde::{Deserialize, Serialize};

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// This is a "cloned" of `lib_circuit_types`'s `Circuit`, but keeping
/// only the fields which are needed for EVALUATION.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub(crate) struct CircuitForEval {
    inputs: Vec<WireRef>,
    gates: Vec<GateForEval>,
    nb_wires: usize,
    nb_outputs: usize,
    display_config: Option<DisplayConfig>,
    metadata: Metadata,
}

/// Basically `impl Circuit`, but without `get_outputs` and `get_wires`
impl CircuitForEval {
    pub(crate) fn get_inputs(&self) -> &[WireRef] {
        &self.inputs
    }

    pub(crate) fn get_config(&self) -> &Option<DisplayConfig> {
        &self.display_config
    }

    pub(crate) fn get_metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub(crate) fn get_nb_wires(&self) -> usize {
        self.nb_wires
    }

    pub(crate) fn get_nb_inputs(&self) -> usize {
        self.inputs.len()
    }

    pub(crate) fn get_nb_outputs(&self) -> usize {
        self.nb_outputs
    }

    pub(crate) fn get_gates(&self) -> &Vec<GateForEval> {
        &self.gates
    }
}

/// Same principle as `CircuitBase` but for `Gate`
/// Reminder: we DO NOT want to send the Gate's type to the client!
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub(crate) struct GateForEval {
    pub(crate) internal: GateTypeForEval,
    /// Gate's output is in practice a Gate's ID or idx
    pub(super) output: WireRef,
}

/// Essentially `enum GateType`, but the fields `gate_type` are simply removed
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub enum GateTypeForEval {
    Binary {
        // TODO SHOULD be rewritten as "is_xor" to support Free XOR [when serializing]
        // gate_type: Option<GateTypeBinary>,
        input_a: WireRef,
        input_b: WireRef,
    },
    Unary {
        // gate_type: Option<GateTypeUnary>,
        input_a: WireRef,
    },
    /// Constant gates (ie 0 and 1) are a special case wrt to parsing the .skcd and garbling/evaluating:
    /// they are "rewritten" using AUX Gate (eg XOR(A,A) = 0, XNOR(A,A) = 1)
    /// That is because contrary to Unary gates, the paper does not explain how to
    /// generalize "Garbling other gate functionalities" to 0 input gate.
    Constant { value: bool },
}

impl GateForEval {
    // TODO move to `impl Gate` directly; and remove `GateInternal`?
    pub(crate) fn get_type(&self) -> &GateTypeForEval {
        &self.internal
    }

    pub(crate) fn get_id(&self) -> usize {
        self.get_output().id
    }

    pub(crate) fn get_output(&self) -> &WireRef {
        &self.output
    }
}

impl From<Circuit> for CircuitForEval {
    fn from(circuit: Circuit) -> Self {
        Self {
            gates: circuit
                .get_gates()
                .iter()
                .map(core::convert::Into::into)
                .collect(),
            nb_wires: circuit.get_nb_wires(),
            inputs: circuit.get_inputs().to_vec(),
            nb_outputs: circuit.get_nb_outputs(),
            display_config: circuit.get_config().map(core::clone::Clone::clone),
            metadata: circuit.get_metadata().clone(),
        }
    }
}

impl From<&Gate> for GateForEval {
    fn from(gate: &Gate) -> Self {
        Self {
            internal: match gate.get_type() {
                GateType::Binary {
                    gate_type: _,
                    input_a,
                    input_b,
                } => GateTypeForEval::Binary {
                    input_a: input_a.clone(),
                    input_b: input_b.clone(),
                },
                GateType::Unary {
                    gate_type: _,
                    input_a,
                } => GateTypeForEval::Unary {
                    input_a: input_a.clone(),
                },
                GateType::Constant { value } => GateTypeForEval::Constant { value: *value },
            },
            output: gate.get_output().clone(),
        }
    }
}
