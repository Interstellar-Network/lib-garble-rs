// #![no_std]
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(elided_lifetimes_in_paths)]

#[cfg(all(not(feature = "std"), feature = "sgx"))]
extern crate sgx_tstd as std;

extern crate alloc;

mod circuit;
mod garble;
mod skcd_parser;
// TODO(interstellar) put behind a feature; the client DOES NOT need it
pub mod ipfs;
pub mod watermark;

// re-export
pub use garble::EncodedGarblerInputs;
pub use garble::EvalCache;
pub use garble::EvaluatorInput;
pub use garble::InterstellarGarbledCircuit;

/// This is the main entry point of this function; meant to be called by the "pallet-ocw-garble"
///
/// It:
/// - parses a .skcd; usually coming from IPFS
/// - garbles it
/// - encode the "garbler inputs" ie the message/watermark/OTP(pinpad or message)
// TODO it SHOULD return a serialized GC, with "encoded inputs"
pub fn garble_skcd(skcd_buf: &[u8]) -> garble::InterstellarGarbledCircuit {
    let circ = circuit::InterstellarCircuit::parse_skcd(skcd_buf).unwrap();

    garble::InterstellarGarbledCircuit::garble(circ)
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::garble_skcd;

    // all_inputs/all_expected_outputs: standard full-adder 2 bits truth table(and expected results)
    // input  i_bit1;
    // input  i_bit2;
    // input  i_carry;
    pub(crate) const FULL_ADDER_2BITS_ALL_INPUTS: &'static [&'static [u16]] = &[
        &[0, 0, 0],
        &[1, 0, 0],
        &[0, 1, 0],
        &[1, 1, 0],
        &[0, 0, 1],
        &[1, 0, 1],
        &[0, 1, 1],
        &[1, 1, 1],
    ];

    // output o_sum;
    // output o_carry;
    pub(crate) const FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS: &'static [&'static [u16]] = &[
        &[0, 0],
        &[1, 0],
        &[1, 0],
        &[0, 1],
        &[1, 0],
        &[0, 1],
        &[0, 1],
        &[1, 1],
    ];

    #[test]
    fn test_garble_full_adder_2bits() {
        let mut garb = garble_skcd(include_bytes!("../examples/data/adder.skcd.pb.bin"));

        for (i, inputs) in FULL_ADDER_2BITS_ALL_INPUTS.iter().enumerate() {
            let outputs = garb.eval(&[], inputs).unwrap();
            let expected_outputs = FULL_ADDER_2BITS_ALL_EXPECTED_OUTPUTS[i];
            println!(
                "inputs = {:?}, outputs = {:?}, expected_outputs = {:?}",
                inputs, outputs, expected_outputs
            );
            assert_eq!(outputs, expected_outputs);
        }
    }

    // NOTE: more tests with "display circuits" are in tests/ folder
}
