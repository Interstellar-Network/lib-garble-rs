use fancy_garbling::circuit::Circuit;
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
}

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
pub fn parse_skcd(buf: &[u8]) -> Result<Circuit, CircuitParserError> {
    let mut buf = &*buf;
    // TODO(interstellar) decode_length_delimited ?
    let skcd: interstellarpbskcd::Skcd = prost::Message::decode(&mut buf).unwrap();
    assert!(
        skcd.a.len() == skcd.b.len()
            && skcd.b.len() == skcd.go.len()
            && skcd.go.len() == skcd.gt.len()
            && skcd.gt.len() == skcd.q.try_into().unwrap(),
        "number of gates inputs/outputs/types DO NOT match!"
    );
    println!("skcd : a = {}", skcd.a.len());

    let mut circ_builder = CircuitBuilder::new();

    // TODO(interstellar) modulus: what should we use??
    let q = 2;

    // We need to use a CircuitRef for Fancy gates(fn xor/fn and/etc)
    // which means we must convert a .skcd GateID(integer) to its corresponding CircuitRef
    let mut map_skcd_gate_id_to_circuit_ref: HashMap<usize, CircuitRef> = HashMap::new();

    // create a vec of [2,2,2..] containing skcd.n elements
    // that is needed for "evaluator_inputs"
    // let mods = vec![2u16; skcd.n.try_into().unwrap()];

    // TODO(interstellar) should we use "garbler_inputs" instead?
    // let inputs = circ_builder.evaluator_inputs(&mods);
    for i in 0..skcd.n as usize {
        // circ.gates.push(Gate::EvaluatorInput { id: i });
        // circ.evaluator_input_refs
        //     .push(CircuitRef { ix: i, modulus: q });

        map_skcd_gate_id_to_circuit_ref.insert(i, circ_builder.evaluator_input(q));
    }

    // We MUST rewrite certain Gate, which means some Gates in .skcd will be converted to several in CircuiBuilder
    // eg OR -> NOT+AND+AND+NOT
    // This means we MUST "correct" the GateID in .skcd by a given offset
    // let mut gate_offset = 0;
    // let mut current_gates = HashSet::new();

    // TODO(interstellar) how should we use skcd's a/b/go?
    for g in 0..skcd.q as usize {
        // TODO(interstellar) gate_offset?
        let skcd_input0 = *skcd.a.get(g).unwrap() as usize;
        let skcd_input1 = *skcd.b.get(g).unwrap() as usize;
        // TODO(interstellar) graph: how to use skcd_output?
        let skcd_output = *skcd.go.get(g).unwrap() as usize;
        let skcd_gate_type = *skcd.gt.get(g).unwrap();
        // println!("Processing gate: {}", g);

        // TODO(interstellar) gate_offset?
        // let default_xref = CircuitRef {
        //     ix: skcd_input0,
        //     modulus: q,
        // };
        // let default_yref = CircuitRef {
        //     ix: skcd_input1,
        //     modulus: q,
        // };

        // TODO(interstellar) apparently "unwrap_or" needed for "display skcd"; why???
        let xref = map_skcd_gate_id_to_circuit_ref.get(&skcd_input0).unwrap();
        // .unwrap_or(&default_xref);
        let yref = map_skcd_gate_id_to_circuit_ref.get(&skcd_input1).unwrap();
        // .unwrap_or(&default_yref);

        // cf "pub trait Fancy"(fancy.rs) for how to build each type of Gate
        match skcd_gate_type.try_into() {
            Ok(SkcdGateType::ZERO) => {
                // if current_gates.insert(circ_builder.constant(0, q).unwrap()) {
                //     gate_offset += 1;
                // }

                // TODO(interstellar) apparently needed for "display skcd"; why???
                map_skcd_gate_id_to_circuit_ref
                    .insert(skcd_output, circ_builder.constant(0, q).unwrap());

                // circ.gates.push(Gate::Constant { val: 0 })
            }
            Ok(SkcdGateType::ONE) => {
                // if current_gates.insert(circ_builder.constant(1, q).unwrap()) {
                //     gate_offset += 1;
                // }

                map_skcd_gate_id_to_circuit_ref
                    .insert(skcd_output, circ_builder.constant(1, q).unwrap());

                // circ.gates.push(Gate::Constant { val: 0 })
            }
            // "Or uses Demorgan's Rule implemented with multiplication and negation."
            Ok(SkcdGateType::OR) => {
                // let x = inputs.get(skcd_input0).unwrap();
                // let y = inputs.get(skcd_input1).unwrap();
                let z = circ_builder.or(&xref, &yref).unwrap();

                // TODO(interstellar) output? circ_builder.output(&z);
                map_skcd_gate_id_to_circuit_ref.insert(skcd_output, z);

                // fn or(&mut self, x: &Self::Item, y: &Self::Item):
                // let notx = self.negate(x)?;
                // let noty = self.negate(y)?;
                // let z = self.and(&notx, &noty)?;
                // self.negate(&z)
                //
                // let notx = fancy_negate(&mut circ, &xref, &oneref);
                // let noty = fancy_negate(&mut circ, &yref, &oneref);
                // // "And is just multiplication, with the requirement that `x` and `y` are mod 2."
                // let z = Gate::Mul {
                //     xref: notx,
                //     yref: noty,
                //     id: id,
                //     // TODO(interstellar)
                //     // out: Some(out),
                //     out: None,
                // };

                // let zref = CircuitRef { ix: id, modulus: q };

                // id += 1;

                // fancy_negate(&mut circ, &zref, &oneref);
            }
            // "Xor is just addition, with the requirement that `x` and `y` are mod 2."
            Ok(SkcdGateType::XOR) => {
                // let x = inputs.get(skcd_input0).unwrap();
                // let y = inputs.get(skcd_input1).unwrap();
                let z = circ_builder.xor(&xref, &yref).unwrap();

                // TODO(interstellar) output? circ_builder.output(&z);
                map_skcd_gate_id_to_circuit_ref.insert(skcd_output, z);

                // circ.gates.push(Gate::Add {
                //     xref,
                //     yref,
                //     out: Some(skcd_output),
                // })

                // fancy_xor(&mut circ, &xref, &yref);
            }
            Ok(SkcdGateType::NAND) => {
                // let x = inputs.get(skcd_input0).unwrap();
                // let y = inputs.get(skcd_input1).unwrap();
                let z = circ_builder.and(&xref, &yref).unwrap();
                let z = circ_builder.negate(&z).unwrap();

                // TODO(interstellar) output? circ_builder.output(&z);
                map_skcd_gate_id_to_circuit_ref.insert(skcd_output, z);

                // "And is just multiplication, with the requirement that `x` and `y` are mod 2."
                // let z = Gate::Mul {
                //     xref: xref,
                //     yref: yref,
                //     id: id,
                //     // TODO(interstellar)
                //     // out: Some(out),
                //     out: None,
                // };

                // let zref = CircuitRef { ix: id, modulus: q };

                // id += 1;

                // fancy_negate(&mut circ, &zref, &oneref);
            }
            _ => todo!(),
        }
    }

    // TODO(interstellar)? parser.rs "Process outputs."
    // IMPORTANT: we MUST use skcd.o to set the CORRECT outputs
    // eg for the 2 bits adder.skcd:
    // - skcd.m = 1
    // - skcd.o = [8,11]
    // -> the 2 CORRECT outputs to be set are: [8,11]
    // If we set the bad ones, we get "FancyError::UninitializedValue" in fancy-garbling/src/circuit.rs at "fn eval"
    // eg L161 etc b/c the cache is not properly set
    // TODO(interstellar) parser.rs proper wires?
    for o in skcd.o {
        let z = map_skcd_gate_id_to_circuit_ref.get(&(o as usize)).unwrap();
        // TODO put that in "outputs_refs" vec? and use it below?
        circ_builder.output(&z).unwrap();

        // circ.output_refs.push(CircuitRef {
        //     // TODO(interstellar) parser.rs proper wires?
        //     // ix: nwires - n3 + i,
        //     ix: i as usize,
        //     modulus: q,
        // });
    }

    Ok(circ_builder.finish())
}
