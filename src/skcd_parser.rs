use crate::circuit::InterstellarCircuit;
use fancy_garbling::circuit::CircuitBuilder;
use fancy_garbling::circuit::CircuitRef;
use fancy_garbling::Fancy;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::convert::TryInto;

// deps/protos/generated/ DOES NOT work b/c it only contains "APIs" and we want circuits/skcd.proto etc
//
// https://github.com/neoeinstein/protoc-gen-prost/issues/26
#[allow(clippy::derive_partial_eq_without_eq)]
mod interstellarpbskcd {
    include!(concat!(env!("OUT_DIR"), "/interstellarpbskcd.rs"));
}

/// All the Gates type possible in SKCD file format
///
/// SHOULD match
/// - "enum SkcdGateType" from skcd.proto
/// - lib_circuits/src/blif/gate_types.h
/// - lib_garble/src/justgarble/gate_types.h
#[derive(Debug)]
enum SkcdGateType {
    ZERO = 0,
    NOR = 1,
    /// A-and-not-B
    AANB = 2,
    /// NOT B
    INVB = 3,
    /// not-A-and-B?
    NAAB = 4,
    /// NOT A
    INV = 5,
    XOR = 6,
    NAND = 7,
    AND = 8,
    XNOR = 9,
    BUF = 10,
    /// A-or-NOT-B?
    AONB = 11,
    BUFB = 12,
    /// NOT-A-or-B?
    NAOB = 13,
    OR = 14,
    ONE = 15,
}

impl TryFrom<i32> for SkcdGateType {
    type Error = ();

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            x if x == SkcdGateType::ZERO as i32 => Ok(SkcdGateType::ZERO),
            x if x == SkcdGateType::NOR as i32 => Ok(SkcdGateType::NOR),
            x if x == SkcdGateType::AANB as i32 => Ok(SkcdGateType::AANB),
            x if x == SkcdGateType::INVB as i32 => Ok(SkcdGateType::INVB),
            x if x == SkcdGateType::NAAB as i32 => Ok(SkcdGateType::NAAB),
            x if x == SkcdGateType::INV as i32 => Ok(SkcdGateType::INV),
            x if x == SkcdGateType::XOR as i32 => Ok(SkcdGateType::XOR),
            x if x == SkcdGateType::NAND as i32 => Ok(SkcdGateType::NAND),
            x if x == SkcdGateType::AND as i32 => Ok(SkcdGateType::AND),
            x if x == SkcdGateType::XNOR as i32 => Ok(SkcdGateType::XNOR),
            x if x == SkcdGateType::BUF as i32 => Ok(SkcdGateType::BUF),
            x if x == SkcdGateType::AONB as i32 => Ok(SkcdGateType::AONB),
            x if x == SkcdGateType::BUFB as i32 => Ok(SkcdGateType::BUFB),
            x if x == SkcdGateType::NAOB as i32 => Ok(SkcdGateType::NAOB),
            x if x == SkcdGateType::OR as i32 => Ok(SkcdGateType::OR),
            x if x == SkcdGateType::ONE as i32 => Ok(SkcdGateType::ONE),
            _ => Err(()),
        }
    }
}

/// Errors emitted by the circuit parser.
#[derive(Debug)]
pub enum CircuitParserError {
    /// An I/O error occurred.
    IoError(std::io::Error),
    /// An error occurred parsing an integer.
    ParseIntError,
    /// An error occurred parsing a line.
    ParseLineError(String),
    /// An error occurred parsing a gate type.
    ParseGateError(String),
    /// InvalidGateIdError: the given GateID from the .skcd DOES NOT match CircuitBuilder's
    InvalidGateIdError(String),
}

impl InterstellarCircuit {
    /// Parse a Protobuf-serialized .skcd file
    /// It is doing what fancy-garbling/src/parser.rs is doing for a "Blif Fashion" txt file,
    /// but for a .skcd instead.
    /// SKCD is essentially the same format, but with the gates written in a different order:
    /// - in "Bilf Fashion": gates are written "gate0_input0 gate0_input1 gate0_output gate0_type" etc
    /// - in SKCD: "gate0_input0 gate1_input0 gate2_input0" etc
    ///
    /// return:
    /// - the graph corresponding to the .skcd(as-is; gates NOT transformed/optimized/etc)
    /// - the list of inputs (gate ids)
    /// - the list of ouputs (gate ids)
    /// [inputs/outputs are needed to walk the graph, and optimize/rewrite if desired]
    pub fn parse_skcd(buf: &[u8]) -> Result<InterstellarCircuit, CircuitParserError> {
        let mut buf = &*buf;
        // TODO(interstellar) decode_length_delimited ?
        let skcd: interstellarpbskcd::Skcd = prost::Message::decode(&mut buf).unwrap();

        let mut circ_builder = CircuitBuilder::new();

        // TODO(interstellar) modulus: what should we use??
        let q = 2;

        let mut skcd_gate_converter = SkcdGateConverter::new();

        // INPUTS
        // IMPORTANT: garbler vs evaluator ones first SHOULD match /lib_circuits/src/blif/blif_parser.cpp "Init the labels"
        // else it makes comparing(debugging) the different (.blif, .skcd, etc) harder
        //
        // TODO!!! ???
        // else the IDs in "map_skcd_gate_id_to_circuit_ref" will not match, and we get "'called `Option::unwrap()` on a `None` value'"
        // in the gate loop just after
        let skcd_config = skcd.config.unwrap();
        let mut input_idx = 0;

        // garbler inputs; same as "evaluator_inputs" above but a bit more complicated b/c how we are about to use
        // them depend on the type (eg 7 segments, i-ching, basic gate inputs for adder, etc)
        let mut skcd_inputs_is_garbled = Vec::<bool>::new();
        for skcd_garbler_input in skcd_config.garbler_inputs {
            for _i in 0..skcd_garbler_input.length {
                skcd_inputs_is_garbled.push(true);
                input_idx += 1;
            }
        }

        for skcd_evaluator_input in skcd_config.evaluator_inputs {
            for _i in 0..skcd_evaluator_input.length {
                skcd_inputs_is_garbled.push(false);
                input_idx += 1;
            }
        }

        assert_eq!(
            input_idx,
            skcd.inputs.len(),
            "inputs and SkcdConfig fields DO NOT match!"
        );

        for (idx, skcd_input) in skcd.inputs.iter().enumerate() {
            let new_gate = if skcd_inputs_is_garbled[idx] {
                circ_builder.garbler_input(q)
            } else {
                circ_builder.evaluator_input(q)
            };
            skcd_gate_converter.insert(skcd_input, new_gate);
        }

        // TODO(interstellar) how should we use skcd's a/b/go?
        for skcd_gate in skcd.gates {
            let xref = skcd_gate_converter.get(&skcd_gate.a);
            let yref = skcd_gate_converter.get(&skcd_gate.b);

            let new_gate = match skcd_gate.r#type.try_into() {
                Ok(SkcdGateType::ZERO) => circ_builder.constant(0, q).unwrap(),
                Ok(SkcdGateType::ONE) => circ_builder.constant(1, q).unwrap(),
                Ok(SkcdGateType::OR) => {
                    // TODO can we use fn proj(&mut self, A: &Wire, q_out: u16, tt: Option<Vec<u16>>)?
                    // let z = circ_builder.proj(&xref, q, Some(vec![0; 4]));
                    circ_builder.or(xref?, yref?).unwrap()
                }
                Ok(SkcdGateType::XOR) => {
                    // TODO can we use fn proj(&mut self, A: &Wire, q_out: u16, tt: Option<Vec<u16>>)?
                    // let z = circ_builder.proj(&xref, q, Some(vec![0; 4]));
                    circ_builder.xor(xref?, yref?).unwrap()
                }
                Ok(SkcdGateType::NAND) => {
                    // TODO can we use fn proj(&mut self, A: &Wire, q_out: u16, tt: Option<Vec<u16>>)?
                    // let z = circ_builder.proj(&xref, q, Some(vec![0; 4]));
                    let z = circ_builder.and(xref?, yref?).unwrap();
                    circ_builder.negate(&z).unwrap()
                }
                Ok(SkcdGateType::INV) => circ_builder.negate(xref?).unwrap(),
                _ => todo!(),
            };

            skcd_gate_converter.insert(&skcd_gate.o, new_gate);
        }

        // IMPORTANT: we MUST use skcd.o to set the CORRECT outputs
        // eg for the 2 bits adder.skcd:
        // - skcd.m = 1
        // - skcd.o = [8,11]
        // -> the 2 CORRECT outputs to be set are: [8,11]
        // If we set the bad ones, we get "FancyError::UninitializedValue" in fancy-garbling/src/circuit.rs at "fn eval"
        // eg L161 etc b/c the cache is not properly set
        for o in skcd.outputs {
            let z = skcd_gate_converter.get(&o).unwrap();
            circ_builder.output(&z).unwrap();
        }

        Ok(InterstellarCircuit {
            circuit: circ_builder.finish(),
        })
    }
}

/// We need to convert soemthing like
/// ".gate XOR  a=rnd[2] b=rnd[0] O=n7016" in the .skcd(which is basically a .blif)
/// into something that CircuitBuilder can accept.
/// Essentially we need to convert a String ID -> CircuitRef(= a usize)
///
/// IMPORTANT
/// For this to work, the INPUTS MUST also go through the same convertion, else
/// when using CircuitBuilder.or/and/etc the CircuitRef WOULD NOT match anything.
/// NOTE that in this case the Circuit still would build fine, but it would fail
/// when eval/garbling.
struct SkcdGateConverter {
    map_skcd_gate_id_to_circuit_ref: HashMap<String, CircuitRef>,
}

impl SkcdGateConverter {
    pub fn new() -> Self {
        Self {
            map_skcd_gate_id_to_circuit_ref: HashMap::new(),
        }
    }

    pub fn get(&self, skcd_gate_id: &str) -> Result<&CircuitRef, CircuitParserError> {
        self.map_skcd_gate_id_to_circuit_ref
            .get(skcd_gate_id)
            .ok_or_else(|| CircuitParserError::InvalidGateIdError(skcd_gate_id.to_string()))
    }

    pub fn insert(&mut self, skcd_gate_id: &str, circuit_ref: CircuitRef) {
        self.map_skcd_gate_id_to_circuit_ref
            .insert(skcd_gate_id.to_string(), circuit_ref);
    }
}
