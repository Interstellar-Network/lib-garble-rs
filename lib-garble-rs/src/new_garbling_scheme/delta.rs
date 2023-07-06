use alloc::vec;

use serde::{Deserialize, Serialize};

use super::{
    block::BlockL,
    block::BlockP,
    constant::{KAPPA, KAPPA_FACTOR},
    wire_labels_set::WireLabelsSet,
    wire_labels_set_bitslice::{WireLabelsSetBitSlice, WireLabelsSetBitsSliceInternal},
    GarblerError,
};
use crate::circuit::{GateType, GateTypeBinary, GateTypeUnary};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub(super) struct Delta {
    block: BlockL,
}

impl Delta {
    /// Build a new `Delta` from the desired `GateType`
    ///
    /// In <https://eprint.iacr.org/2021/739.pdf>
    /// this is the main loop of "Algorithm 5 Gate" up to line 17/18
    ///
    /// Compute the ∇ = f1.1 in the paper
    /// "Collapse.
    /// These four outputs of the random oracle are given to f1,1 to produce
    /// ∇ (this is either ∇⊕ or ∇∧, depending on the gate type)"
    ///
    /// - fill the standard truth table for your gate
    ///     - Binary gates: 2 columns and 4 rows
    ///     - unary gates: 1 column and 2 rows
    /// - write the TRANSPOSE and COMPLEMENT of the Truth Table
    /// - in the "Delta Table" with either 16 rows + 4 cols for Binary or 4 rows + 2 cols [on the left of ∇ col]
    ///     - set ∇ = 1 for first and last row
    ///     - set ∇ = 1 for TRANSPOSE
    ///     - set ∇ = 1 for COMPLEMENT
    ///     - you SHOULD always have 4 rows with ∇ = 1 for both Binary and Unary (and others) gates!
    /// - next "group" the COLUMNS Sxy (or Sx for Unary) by their value
    ///     - you SHOULD identify two different possible values, and only 2!
    ///     - you CAN have two groups of two, or 1 group of 1 and one group of 3; it depends
    ///     - set the appropriate L0 and L1 based on the groups and truth table
    ///     - NOTE: if a group has more than on Sxy/Sx column with a given value (eg S01 and S00) you can pick whichever
    ///       you want; what matters is to be determistic b/w garbling and evaluating (ie use the same one!)
    ///
    pub(super) fn new(
        compressed_set: &WireLabelsSet,
        gate_type: &GateType,
    ) -> Result<(BlockP, BlockP, Self), GarblerError> {
        // "5: initialize ∇g ← 0ℓ′ and let j = 1"
        // "Next, the random oracle outputs (Xg00, Xg01, Xg10, Xg11) are used to derive a
        // single ℓg -bit string ∇g (that is padded by 0s to make its length equal to ℓ′)"
        // -> Implies that only the l first bits of ∇g are potentially set??
        let mut delta_g_block = BlockP::new_zero();

        // Return the (x00,x01,x10,x11) values for which the delta colmun == 1
        // eg for AND it will return {0000, 0001, 1110, 1111}
        // and for XOR {0000, 1001, 0110, 1111}
        // NOTE: the set will be definition always contain {0000, 1111}
        // the other 2 elements will depend on the truth table
        let truth_table = TruthTable::new_from_gate(gate_type);
        let mut delta_slices = vec![
            WireLabelsSetBitSlice::new_binary_gate_from_bool(false, false, false, false),
            truth_table.truth_table.clone(),
            truth_table.get_complement(),
            WireLabelsSetBitSlice::new_binary_gate_from_bool(true, true, true, true),
        ];

        // TODO for performance; this should be rewrittten/vectorized?
        let mut count_bits_ones = 0;
        for j in 0..KAPPA * KAPPA_FACTOR {
            let slice = compressed_set.get_bits_slice(j)?;

            if delta_slices.contains(&slice) {
                delta_g_block.set_bit(j);
                count_bits_ones += 1;
            }

            // "14: until HW (∇g ) = ℓ or j = ℓ"
            if count_bits_ones == KAPPA {
                break;
            }
        }

        // "15: if HW (∇g )̸ = ℓ then"
        if count_bits_ones != KAPPA {
            // "16: ABORT the computation"
            return Err(GarblerError::BadHammingWeight {
                hw: count_bits_ones,
            });
        }

        // Following are after line 19: of "Algorithm 5 Gate"
        //
        // To know how to "project" cf docstring
        //
        // NOTE: both `CompressedSet`(randomly generated) and `Delta` are `BlockP`
        // NOTE: `Delta` is technically a `BlockL` padded to a `BlockP`(?)
        // TODO? but we want a `BlockL`
        // TODO same issue with `l1`
        #[allow(clippy::match_same_arms)]
        let (l0_full, l1_full) = match gate_type {
            GateType::Binary {
                gate_type: r#type,
                input_a: _,
                input_b: _,
            } => match r#type {
                Some(GateTypeBinary::XOR) => (
                    BlockP::new_projection(compressed_set.get_x00(), &delta_g_block),
                    BlockP::new_projection(compressed_set.get_x01(), &delta_g_block),
                ),
                Some(GateTypeBinary::XNOR) => (
                    BlockP::new_projection(compressed_set.get_x01(), &delta_g_block),
                    BlockP::new_projection(compressed_set.get_x00(), &delta_g_block),
                ),
                Some(GateTypeBinary::AND) => (
                    BlockP::new_projection(compressed_set.get_x00(), &delta_g_block),
                    BlockP::new_projection(compressed_set.get_x11(), &delta_g_block),
                ),
                Some(GateTypeBinary::NAND) => (
                    BlockP::new_projection(compressed_set.get_x11(), &delta_g_block),
                    BlockP::new_projection(compressed_set.get_x00(), &delta_g_block),
                ),
                Some(GateTypeBinary::OR) => (
                    BlockP::new_projection(compressed_set.get_x00(), &delta_g_block),
                    BlockP::new_projection(compressed_set.get_x01(), &delta_g_block),
                ),
                Some(GateTypeBinary::NOR) => (
                    BlockP::new_projection(compressed_set.get_x01(), &delta_g_block),
                    BlockP::new_projection(compressed_set.get_x00(), &delta_g_block),
                ),
                // GateTypeBinary is None only when deserializing
                None => unimplemented!("Delta::new for None[GateTypeBinary]!"),
            },
            GateType::Unary {
                gate_type: r#type,
                input_a: _,
            } => match r#type {
                // TODO(opt); probably not needed if we don't use it in `evaluate_internal`
                // but it's never called since "free BUF/NOT" so it should not matter
                Some(GateTypeUnary::INV) => (
                    BlockP::new_projection(compressed_set.get_x1(), &delta_g_block),
                    BlockP::new_projection(compressed_set.get_x0(), &delta_g_block),
                ),
                Some(GateTypeUnary::BUF) => (
                    BlockP::new_projection(compressed_set.get_x0(), &delta_g_block),
                    BlockP::new_projection(compressed_set.get_x1(), &delta_g_block),
                ),
                // GateTypeUnary is None only when deserializing
                None => unimplemented!("Delta::new for None[GateTypeUnary]!"),
            },
            // [constant gate special case]
            // They SHOULD have be "rewritten" to AUX(eg XNOR) gates by the `skcd_parser`
            GateType::Constant { value: _ } => {
                unimplemented!("Delta::new for Constant gates is a special case!")
            }
        };

        let delta = Self {
            block: delta_g_block.into(),
        };

        // cf `Wire::new` assert for why this is bad
        assert!(l0_full != l1_full, "`L0` and `L1` MUST be different!");
        Ok((l0_full, l1_full, delta))
    }

    pub(super) fn get_block(&self) -> &BlockL {
        &self.block
    }
}

////////////////////////////////////////////////////////////////////////////////
// Below: was trying to make the Delta dynamic instead of hardcoding the
// few types of Gates we need...

// use core::array;

// use crate::garble::new_garbling_scheme::{
//     block::KAPPA_BYTES,
//     constant::{KAPPA, KAPPA_FACTOR},
//     random_oracle, Gate, GateInternal, GateType, WireValue,
// };
// use hashbrown::HashMap;
// use itertools::Itertools;

// use super::{block::BlockP, CompressedSet, CompressedSetBitSlice};

// mod delta_row {
//     use crate::garble::new_garbling_scheme::{CompressedSetBitSlice, WireValue};

//     /// Represent a ROW in the "delta table"
//     /// X00 X01 X10 X11 ∇ S00 S01 S10 S11
//     // TODO this is probably dup with Wire and/or Block
//     #[derive(Debug, PartialEq, Default)]
//     pub(super) struct DeltaRow {
//         x00: WireValue,
//         x01: WireValue,
//         x10: WireValue,
//         x11: WireValue,
//         delta: WireValue,
//         // NOTE: technically we DO NOT need to store these b/c they only depend
//         // on if delta is set, and x00,...
//         s00: WireValue,
//         s01: WireValue,
//         s10: WireValue,
//         s11: WireValue,
//     }

//     impl DeltaRow {
//         pub(super) fn new(x00: WireValue, x01: WireValue, x10: WireValue, x11: WireValue) -> Self {
//             Self {
//                 x00,
//                 x01,
//                 x10,
//                 x11,
//                 ..Default::default()
//             }
//         }

//         pub(super) fn get_Xab(&self) -> CompressedSetBitSlice {
//             CompressedSetBitSlice {
//                 x00: self.x00.clone(),
//                 x01: self.x01.clone(),
//                 x10: self.x10.clone(),
//                 x11: self.x11.clone(),
//             }
//         }

//         pub(super) fn get_delta(&self) -> &WireValue {
//             &self.delta
//         }

//         pub(super) fn get_x00(&self) -> &WireValue {
//             &self.x00
//         }

//         pub(super) fn get_s00(&self) -> &WireValue {
//             &self.s00
//         }

//         pub(super) fn get_s01(&self) -> &WireValue {
//             &self.s01
//         }

//         pub(super) fn get_s10(&self) -> &WireValue {
//             &self.s10
//         }

//         pub(super) fn get_s11(&self) -> &WireValue {
//             &self.s11
//         }

//         /// Both:
//         /// - set delta = true for the current DeltaRow
//         /// - AND "project" (x00,x01,x10,x11) -> (s00,s01,s10,s11)
//         ///   ie copy (x00,...) to (s00,...)
//         pub(super) fn set_delta_true(&mut self) {
//             self.delta.value = true;
//             self.s00 = self.x00.clone();
//             self.s01 = self.x01.clone();
//             self.s10 = self.x10.clone();
//             self.s11 = self.x11.clone();
//         }

//         #[cfg(test)]
//         pub(super) fn set_x00_delta(&mut self, x00: WireValue, delta: WireValue) {
//             self.x00 = x00;
//             self.delta = delta;
//         }
//     }
// }

// fn vec_bool_to_u16(bits: &[bool]) -> u16 {
//     assert_eq!(
//         bits.len(),
//         16,
//         "The input Vec<bool> must have exactly 16 elements."
//     );
//     let mut value: u16 = 0;
//     for (index, &bit) in bits.iter().enumerate() {
//         if bit {
//             value |= 1 << index;
//         }
//     }
//     value
// }

// ///
// pub(super) struct Delta {
//     block: BlockP,
// }

// impl Delta {
//     /// Build a new `Delta` from a properly initialized `DeltaTable`
//     ///
//     /// In https://eprint.iacr.org/2021/739.pdf
//     /// this is the main loop of "Algorithm 5 Gate" up to line 17/18
//     pub(super) fn new_from_delta_table(
//         delta_table: &DeltaTable,
//         compressed_set: &CompressedSet,
//         gate: &Gate,
//     ) -> Self {
//         assert!(
//             delta_table.is_ready(),
//             "new_from_delta_table MUST be called AFTER step4_set_for_gate!"
//         );

//         // "5: initialize ∇g ← 0ℓ′ and let j = 1"
//         // BUT!
//         // "Next, the random oracle outputs (Xg00, Xg01, Xg10, Xg11) are used to derive a
//         // single ℓg -bit string ∇g (that is padded by 0s to make its length equal to ℓ′) that
//         // encodes the gate functionality."
//         let mut delta_g_block = BlockP::new_zero();

//         let delta_slices = delta_table.get_delta_slices();

//         // TODO for performance; this should be rewrittten/vectorized?
//         for j in 0..KAPPA * KAPPA_FACTOR {
//             let slice = compressed_set.get_bits_slice(j);

//             if delta_slices.contains(&slice) {
//                 delta_g_block.set_bit(j);
//             }
//         }

//         Self {
//             block: delta_g_block,
//         }
//     }

//     pub(super) fn get_block(&self) -> &BlockP {
//         &self.block
//     }
// }

// /// Represent a "Delta table" in
// /// in https://eprint.iacr.org/2021/739.pdf
// /// cf "Additional Details of the Scheme"
// /// This is NOT ∇ itself!
// pub(super) struct DeltaTable {
//     /// Rows: 16 because we have 4 "bits": s00,s01,s10,s11
//     rows: [delta_row::DeltaRow; 16],
//     /// We use this field mostly as a "is_ready" flag
//     /// It SHOULD be set by "step4_set_for_gate" to mark the table as ready for "compute_s1"
//     gate_type: Option<GateType>,
// }

// impl DeltaTable {
//     /// Build a new `DeltaTable` for the given `Gate`(or rather `GateType`).
//     ///
//     /// "B Garbling Other Gates"
//     /// "(iii) With Table 1 as a template, initialize a new 16-row table T ,
//     /// whose index is the vector [X00, X01, X10, X11] and its value is ∇. Initialize all ∇
//     /// values to 0 (i.e., T [X00, X01, X10, X11] = 0 for all X00, X01, X10, and X11);"
//     pub(super) fn new_for_gate(gate: &Gate) -> Self {
//         /// this will be the vector of X00 X01 X10 X11
//         /// 0000
//         /// 0001
//         /// 0010
//         /// ...
//         /// -> so 16 rows
//         let mut delta_rows = Vec::with_capacity(16);
//         for x00 in 0..2 {
//             for x01 in 0..2 {
//                 for x10 in 0..2 {
//                     for x11 in 0..2 {
//                         // "!= 0" is just to convert integer -> bool
//                         delta_rows.push(delta_row::DeltaRow::new(
//                             WireValue { value: x00 != 0 },
//                             WireValue { value: x01 != 0 },
//                             WireValue { value: x10 != 0 },
//                             WireValue { value: x11 != 0 },
//                         ));
//                     }
//                 }
//             }
//         }

//         let mut delta_table = Self {
//             rows: delta_rows.try_into().unwrap(),
//             gate_type: None,
//         };

//         delta_table.step4_set_for_gate(gate);

//         assert!(delta_table.is_ready());

//         delta_table
//     }

//     fn is_ready(&self) -> bool {
//         self.gate_type.is_some()
//     }

//     /// "(iv) Set ∇ = 1 in the rows indexed by the vectors from Step (ii), as well as the first and last rows."
//     fn step4_set_for_gate(&mut self, gate: &Gate) {
//         let truth_table = TruthTable::new_from_gate(gate.internal.get_type());

//         // "Set ∇ = 1 in the rows indexed by the vectors from Step (ii)"
//         for row in self.rows.iter_mut() {
//             if row.get_Xab() == truth_table.truth_table
//                 || row.get_Xab() == truth_table.get_complement()
//             {
//                 row.set_delta_true();
//             }
//         }
//         // "as well as the first and last rows."
//         self.rows[0].set_delta_true();
//         self.rows[15].set_delta_true();

//         self.gate_type = Some(gate.internal.get_type().clone());
//     }

//     /// Return the (x00,x01,x10,x11) values for which delta == 1
//     /// eg for AND it will return {0000, 0001, 1110, 1111}
//     /// and for XOR {0000, 1001, 0110, 1111}
//     /// For a standard gate, it SHOULD return 4 elements.
//     /// For non-binary one(if applicable) it should return only 2(the first and last row)?
//     ///
//     /// This is a helper for "Algorithm 5 Gate" line 8: and 10:
//     fn get_delta_slices(&self) -> Vec<CompressedSetBitSlice> {
//         self.rows
//             .iter()
//             .filter(|row| row.get_delta().value)
//             .map(|delta_row| delta_row.get_Xab())
//             .collect()
//     }

//     // In the papers:
//     // A ◦ B = projection of A[i] for positions with B[i] = 1
//     // also noted & as in: S0 = X00 & ∇
//     //
//     // Return:
//     // other_vec[i] for all positions of "self" where self[i] = 1
//     // == other_vec & self
//     // == other_vec ◦ self
//     // fn project_x00_delta(&self) -> Vec<WireValue> {
//     //     self.rows
//     //         .iter()
//     //         .map(|delta_row: &delta_row::DeltaRow| delta_row.get_x00())
//     //         .into_iter()
//     //         .zip(self.rows.iter().map(|delta_row| delta_row.get_delta()))
//     //         .map(|(x00, delta)| if delta { x00.clone() } else { false })
//     //         .collect()
//     // }

//     // Compute "s1"
//     // ie project the appropriate X0/X01/.. onto "delta" based on the current gate's truth table
//     // or "delta table" in the paper
//     //
//     // IMPORTANT: "step4_set_for_gate" SHOULD have been called before this!
//     // fn compute_s1(&self) -> Vec<WireValue> {
//     //     assert!(
//     //         self.is_ready(),
//     //         "compute_s1 MUST be called AFTER step4_set_for_gate!"
//     //     );

//     //     // "the right side demonstrates how combining Xij [j]&∇ collapses into only two distinct values"
//     //     // Let's check!
//     //     let counts = {
//     //         let mut s00_col = vec![];
//     //         let mut s01_col = vec![];
//     //         let mut s10_col = vec![];
//     //         let mut s11_col = vec![];
//     //         for delta_row in self.rows.iter() {
//     //             s00_col.push(delta_row.get_s00());
//     //             s01_col.push(delta_row.get_s01());
//     //             s10_col.push(delta_row.get_s10());
//     //             s11_col.push(delta_row.get_s11());
//     //         }
//     //         let s_cols = vec![s00_col, s01_col, s10_col, s11_col];

//     //         let mut map_counts: HashMap<u16, usize> = HashMap::new();
//     //         let s_cols_u16: Vec<u16> = s_cols.iter().map(|s_col| vec_bool_to_u16(s_col)).collect();
//     //         for s_val_u16 in s_cols_u16 {
//     //             let mut values = map_counts.entry(s_val_u16).or_default();
//     //             *values += 1;
//     //         }
//     //         println!("compute_s1: counts: {:?}", map_counts);
//     //         // "only two distinct values"
//     //         assert_eq!(map_counts.len(), 2, "SHOULD only have 2 distinct values!");

//     //         map_counts
//     //     };

//     //     let gate_type = self.gate_type.as_ref().unwrap();
//     //     match gate_type {
//     //         // GateType::ZERO => todo!(),
//     //         // GateType::INV => todo!(),
//     //         GateType::XOR => todo!(),
//     //         GateType::AND => todo!(),
//     //         // GateType::ONE => todo!(),
//     //     }
//     //     todo!()
//     // }
// }

/// Represent the truth table for a 2 inputs boolean gate
/// ordered classically as: 00, 01, 10, 11
struct TruthTable {
    truth_table: WireLabelsSetBitSlice,
}

impl TruthTable {
    pub(self) fn new_from_gate(gate_type: &GateType) -> Self {
        // TODO or instead of handling 1-input and constant gates here -> rewrite all of these in skcd_parser.rs?
        match gate_type {
            // GateType::ZERO => todo!(),
            // GateType::NOR => TruthTable {
            //     truth_table: [true, false, false, false],
            // },
            // GateType::AANB => todo!(),
            // GateType::INVB => todo!(),
            // GateType::NAAB => todo!(),
            // TODO? NOR(A, A) inverts the input A.
            // GateType::INV => todo!(),
            GateType::Binary {
                gate_type: r#type,
                input_a: _,
                input_b: _,
            } => match r#type {
                Some(GateTypeBinary::XOR) => TruthTable {
                    truth_table: WireLabelsSetBitSlice::new_binary_gate_from_bool(
                        false, true, true, false,
                    ),
                },
                Some(GateTypeBinary::NAND) => TruthTable {
                    truth_table: WireLabelsSetBitSlice::new_binary_gate_from_bool(
                        true, true, true, false,
                    ),
                },
                Some(GateTypeBinary::AND) => TruthTable {
                    truth_table: WireLabelsSetBitSlice::new_binary_gate_from_bool(
                        false, false, false, true,
                    ),
                },
                Some(GateTypeBinary::OR) => TruthTable {
                    truth_table: WireLabelsSetBitSlice::new_binary_gate_from_bool(
                        false, true, true, true,
                    ),
                },
                Some(GateTypeBinary::NOR) => TruthTable {
                    truth_table: WireLabelsSetBitSlice::new_binary_gate_from_bool(
                        true, false, false, false,
                    ),
                },
                Some(GateTypeBinary::XNOR) => TruthTable {
                    truth_table: WireLabelsSetBitSlice::new_binary_gate_from_bool(
                        true, false, false, true,
                    ),
                },
                // GateTypeBinary is None only when deserializing
                None => unimplemented!("TruthTable for None[GateTypeBinary]!"),
            },
            GateType::Unary {
                gate_type: r#type,
                input_a: _,
            } => match r#type {
                Some(GateTypeUnary::INV) => TruthTable {
                    truth_table: WireLabelsSetBitSlice::new_unary_gate_from_bool(false, true),
                },
                Some(GateTypeUnary::BUF) => TruthTable {
                    truth_table: WireLabelsSetBitSlice::new_unary_gate_from_bool(true, false),
                },
                // GateTypeUnary is None only when deserializing
                None => unimplemented!("TruthTable for None[GateTypeUnary]!"),
            },
            // [constant gate special case]
            // They SHOULD have be "rewritten" to AUX(eg XNOR) gates by the `skcd_parser`
            GateType::Constant { value: _ } => {
                unimplemented!("TruthTable for Constant gates is a special case!")
            }
        }
    }

    pub(self) fn get_complement(&self) -> WireLabelsSetBitSlice {
        match &self.truth_table.internal {
            WireLabelsSetBitsSliceInternal::BinaryGate { x00, x01, x10, x11 } => {
                WireLabelsSetBitSlice::new_binary_gate_from_bool(
                    !x00.value, !x01.value, !x10.value, !x11.value,
                )
            }
            WireLabelsSetBitsSliceInternal::UnaryGate { x0, x1 } => {
                WireLabelsSetBitSlice::new_unary_gate_from_bool(!x0.value, !x1.value)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::rngs::ThreadRng;
    use rand::Rng;

    /// Minimal Reprodocible Example for Delta for a NAND Gate
    /// Helpful to visualize of the algorithm works if we hardcoded all the truth tables etc
    ///
    /// For this we use l = 16 and `l_prime` = 64
    /// Techinically not OK vs the security parameter but does not really matter here.
    ///
    fn mre_delta_binary_gate_aux() {
        let mut rng = rand::thread_rng();

        let x00 = rand_array_16(&mut rng);
        let x01 = rand_array_16(&mut rng);
        let x10 = rand_array_16(&mut rng);
        let x11 = rand_array_16(&mut rng);
        println!("{x00:?}\n{x01:?}\n{x10:?}\n{x11:?}\n");

        // Delta: init with 0; and longer than X00 etc
        // Implicitely means it will contain mostly 0, except for the start length which matches with X00 etc
        let mut delta = [0u8; 64];

        let delta_slices = [
            // This first one is hardcoded
            [0u8, 0, 0, 0],
            // The two middle ones are the truth table for the current Gate type
            // and its complement
            // // NAND Gate
            // [1, 0, 0, 0],
            // [0, 1, 1, 1],
            // XOR Gate
            [0, 1, 1, 0],
            [1, 0, 0, 1],
            // This last one is also hardcoded
            [1, 1, 1, 1],
        ];

        for i in 0..x00.len() {
            let current_slice = [x00[i], x01[i], x10[i], x11[i]];
            println!("current_slice : {current_slice:?}");

            if delta_slices.contains(&current_slice) {
                println!("match!");
                delta[i] = 1;
            }
        }

        println!("delta : {delta:?}");

        // Build the L0 and L1
        // The right side is always `Delta`, but the left DEPEND on the current Gate type
        // // NAND Gate
        // let l0 = new_projection(&x10, &delta);
        // let l1 = new_projection(&x00, &delta);
        // XOR Gate
        let l0 = new_projection(&x00, &delta);
        let l1 = new_projection(&x01, &delta);
        println!("l0 : {l0:?}\nl1 : {l1:?}\n");
        // cf `Wire::new` for why this assert matters!
        assert_ne!(l0, l1, "L0 and L1 MUST NOT be the same!");
    }

    #[test]
    #[ignore]
    fn mre_delta_nand_gate() {
        for _i in 0..1000 {
            mre_delta_binary_gate_aux()
        }
    }

    fn rand_array_16(rng: &mut ThreadRng) -> [u8; 16] {
        let mut arr = [0u8; 16];
        for i in 0..16 {
            let r: u8 = rng.gen();
            arr[i] = u8::from(r > 127);
        }

        arr
    }

    /// cf `BlockP::new_projection`
    /// "A ◦ B = projection of A[i] for positions with B[i] = 1"
    fn new_projection(left: &[u8], right: &[u8]) -> [u8; 16] {
        let mut res = [0u8; 16];

        for (idx, bit) in right.iter().enumerate() {
            if *bit >= 1 {
                res[idx] = left[idx];
            }
        }

        res
    }

    //     use super::*;
    //     use crate::garble::new_garbling_scheme::*;
    //     use crate::garble::InterstellarCircuit;

    //     /// Not really a useful test
    //     /// It only checks the "truth table" of the Xab cols is generated in order:
    //     /// 0 0 0 0
    //     /// 0 0 0 1
    //     /// 0 0 1 0
    //     /// 0 0 1 1
    //     /// ...
    //     /// 1 1 1 1
    //     #[test]
    //     fn test_delta_table_Xab() {
    //         // NOTE: for this we only care about the first 4 cols; so the GateType does not matter
    //         let gate = Gate {
    //             internal: GateInternal::Standard {
    //                 r#type: GateType::AND,
    //                 input_a: None,
    //                 input_b: None,
    //             },
    //             output: WireRef { id: 1 },
    //         };
    //         let delta_table = DeltaTable::new_for_gate(&gate);

    //         assert_eq!(delta_table.rows[0].get_Xab(), [false, false, false, false]);
    //         assert_eq!(delta_table.rows[1].get_Xab(), [false, false, false, true,]);
    //         assert_eq!(delta_table.rows[10].get_Xab(), [true, false, true, false,]);
    //         assert_eq!(delta_table.rows[15].get_Xab(), [true, true, true, true,]);
    //     }

    //     #[test]
    //     fn test_delta_table_AND() {
    //         let gate = Gate {
    //             internal: GateInternal::Standard {
    //                 r#type: GateType::AND,
    //                 input_a: None,
    //                 input_b: None,
    //             },
    //             output: WireRef { id: 1 },
    //         };
    //         let delta_table = DeltaTable::new_for_gate(&gate);

    //         assert_eq!(delta_table.rows[0].get_delta(), true);
    //         assert_eq!(delta_table.rows[1].get_delta(), true);
    //         assert_eq!(delta_table.rows[14].get_delta(), true);
    //         assert_eq!(delta_table.rows[15].get_delta(), true);
    //         assert_eq!(
    //             delta_table
    //                 .rows
    //                 .iter()
    //                 .filter(|delta_row| !delta_row.get_delta().value)
    //                 .count(),
    //             12,
    //             "delta table: `false` rows count does not match!"
    //         );
    //     }

    //     #[test]
    //     fn test_delta_table_XOR() {
    //         let gate = Gate {
    //             internal: GateInternal::Standard {
    //                 r#type: GateType::XOR,
    //                 input_a: None,
    //                 input_b: None,
    //             },
    //             output: WireRef { id: 1 },
    //         };
    //         let delta_table = DeltaTable::new_for_gate(&gate);

    //         assert_eq!(delta_table.rows[0].get_delta(), true);
    //         assert_eq!(delta_table.rows[6].get_delta(), true);
    //         assert_eq!(delta_table.rows[9].get_delta(), true);
    //         assert_eq!(delta_table.rows[15].get_delta(), true);
    //         assert_eq!(
    //             delta_table
    //                 .rows
    //                 .iter()
    //                 .filter(|delta_row| !delta_row.get_delta().value)
    //                 .count(),
    //             12,
    //             "delta table: `false` rows count does not match!"
    //         );
    //     }

    //     // TODO tests for `new_from_delta_table`

    //     // #[test]
    //     // fn test_project_x00_delta_all_1() {
    //     //     let mut delta_table = DeltaTable::new_default();
    //     //     for delta_row in delta_table.rows.iter_mut() {
    //     //         delta_row.set_x00_delta(true, true);
    //     //     }

    //     //     let res = delta_table.project_x00_delta();

    //     //     assert!(res.iter().all(|&e| e));
    //     // }

    //     // #[test]
    //     // fn test_project_x00_delta_10() {
    //     //     let mut delta_table = DeltaTable::new_default();
    //     //     for delta_row in delta_table.rows.iter_mut() {
    //     //         delta_row.set_x00_delta(true, false);
    //     //     }

    //     //     let res = delta_table.project_x00_delta();

    //     //     assert!(res.iter().all(|&e| !e));
    //     // }

    //     // #[test]
    //     // fn test_project_x00_delta_01() {
    //     //     let mut delta_table = DeltaTable::new_default();
    //     //     for delta_row in delta_table.rows.iter_mut() {
    //     //         delta_row.set_x00_delta(false, true);
    //     //     }

    //     //     let res = delta_table.project_x00_delta();

    //     //     assert!(res.iter().all(|&e| !e));
    //     // }

    //     // #[test]
    //     // fn test_compute_s1() {
    //     //     let gate = Gate {
    //     //         internal: GateInternal::Standard {
    //     //             r#type: GateType::AND,
    //     //             input_a: None,
    //     //             input_b: None,
    //     //         },
    //     //         output: WireRef { id: 1 },
    //     //     };
    //     //     let mut delta_table = DeltaTable::new_for_gate(&gate);

    //     //     assert_eq!(delta_table.rows[0].get_delta(), true);
    //     //     assert_eq!(delta_table.rows[1].get_delta(), true);
    //     //     assert_eq!(delta_table.rows[14].get_delta(), true);
    //     //     assert_eq!(delta_table.rows[15].get_delta(), true);
    //     // }
}
