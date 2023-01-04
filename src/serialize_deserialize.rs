use crate::InterstellarGarbledCircuit;
use alloc::collections::BTreeMap;
use prost::Message;

// derive_partial_eq_without_eq: https://github.com/neoeinstein/protoc-gen-prost/issues/26
#[allow(clippy::derive_partial_eq_without_eq)]
mod interstellarpbgarbled {
    // TODO(interstellar) can we use prost-build(and prost-derive) in SGX env?
    // include!(concat!(env!("OUT_DIR"), "/interstellarpbgarbled.rs"));
    include!("../deps/protos/generated/rust/interstellarpbgarbled.rs");
}

impl InterstellarGarbledCircuit {
    // TODO finalize(ie use the real data from "self") AND add tests!
    pub fn serialize(&self) -> Vec<u8> {
        // if there is no "display_config"(ie == None) the whole "config" field should be None
        // It means it is a "generic circuit" not a "display circuit"
        let config: Option<interstellarpbgarbled::EvaluatorConfig> =
            if let Some(display_config) = self.config.display_config {
                Some(interstellarpbgarbled::EvaluatorConfig {
                    width: display_config.width,
                    height: display_config.height,
                })
            } else {
                None
            };

        let msg = interstellarpbgarbled::InterstellarGarbledCircuit {
            garbled_circuit: Some(interstellarpbgarbled::FancyGarblingClassicGarbledCircuit {}),
            encoder: Some(interstellarpbgarbled::FancyGarblingClassicEncoder {
                evaluator_inputs: vec![interstellarpbgarbled::FancyGarblingWire {
                    block: Some(interstellarpbgarbled::fancy_garbling_wire::Block::Wire(
                        interstellarpbgarbled::FancyGarblingWireMod2 {
                            block: Some(interstellarpbgarbled::ScuttlebuttBlock {
                                high: 0,
                                low: 0,
                            }),
                        },
                    )),
                }],
                deltas: BTreeMap::default(),
            }),
            config: config,
        };

        msg.encode_to_vec()
    }
}
