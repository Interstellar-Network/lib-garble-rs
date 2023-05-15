use crate::garble::new_garbling_scheme::{
    block::KAPPA_BYTES,
    constant::{KAPPA, KAPPA_FACTOR},
    random_oracle, Gate, GateInternal, GateType, WireInternal,
};
use hashbrown::HashMap;
use itertools::Itertools;

use super::{block::BlockP, CompressedSet};

mod delta_row {
    use crate::garble::new_garbling_scheme::WireInternal;

    /// Represent a ROW in the "delta table"
    /// X00 X01 X10 X11 ∇ S00 S01 S10 S11
    // TODO this is probably dup with Wire and/or Block
    #[derive(Debug, PartialEq, Default)]
    pub(super) struct DeltaRow {
        x00: WireInternal,
        x01: WireInternal,
        x10: WireInternal,
        x11: WireInternal,
        delta: WireInternal,
        // NOTE: technically we DO NOT need to store these b/c they only depend
        // on if delta is set, and x00,...
        s00: WireInternal,
        s01: WireInternal,
        s10: WireInternal,
        s11: WireInternal,
    }

    impl DeltaRow {
        pub(super) fn new(
            x00: WireInternal,
            x01: WireInternal,
            x10: WireInternal,
            x11: WireInternal,
        ) -> Self {
            Self {
                x00,
                x01,
                x10,
                x11,
                ..Default::default()
            }
        }

        pub(super) fn get_Xab(&self) -> [WireInternal; 4] {
            [self.x00, self.x01, self.x10, self.x11]
        }

        pub(super) fn get_delta(&self) -> WireInternal {
            self.delta
        }

        pub(super) fn get_x00(&self) -> WireInternal {
            self.x00
        }

        pub(super) fn get_s00(&self) -> WireInternal {
            self.s00
        }

        pub(super) fn get_s01(&self) -> WireInternal {
            self.s01
        }

        pub(super) fn get_s10(&self) -> WireInternal {
            self.s10
        }

        pub(super) fn get_s11(&self) -> WireInternal {
            self.s11
        }

        /// Both:
        /// - set delta = true for the current DeltaRow
        /// - AND "project" (x00,x01,x10,x11) -> (s00,s01,s10,s11)
        ///   ie copy (x00,...) to (s00,...)
        pub(super) fn set_delta_true(&mut self) {
            self.delta = true;
            self.s00 = self.x00;
            self.s01 = self.x01;
            self.s10 = self.x10;
            self.s11 = self.x11;
        }

        #[cfg(test)]
        pub(super) fn set_x00_delta(&mut self, x00: WireInternal, delta: WireInternal) {
            self.x00 = x00;
            self.delta = delta;
        }
    }
}

fn vec_bool_to_u16(bits: &[bool]) -> u16 {
    assert_eq!(
        bits.len(),
        16,
        "The input Vec<bool> must have exactly 16 elements."
    );
    let mut value: u16 = 0;
    for (index, &bit) in bits.iter().enumerate() {
        if bit {
            value |= 1 << index;
        }
    }
    value
}

///
pub(super) struct Delta {
    block: BlockP,
}

impl Delta {
    /// Build a new `Delta` from a properly initialized `DeltaTable`
    ///
    /// In https://eprint.iacr.org/2021/739.pdf
    /// this is the main loop of "Algorithm 5 Gate" up to line 17/18
    pub(super) fn new_from_delta_table(
        delta_table: DeltaTable,
        compressed_set: &CompressedSet,
    ) -> Self {
        assert!(
            delta_table.is_ready(),
            "new_from_delta_table MUST be called AFTER step4_set_for_gate!"
        );

        // "5: initialize ∇g ← 0ℓ′ and let j = 1"
        let block = BlockP::new_with2([0; KAPPA_BYTES * KAPPA_FACTOR]);

        for j in 0..KAPPA * KAPPA_FACTOR {
            let slice = compressed_set.get_bits_slice(j);
        }

        Self { block: todo!() }
    }
}

/// Represent a "Delta table" in
/// in https://eprint.iacr.org/2021/739.pdf
/// cf "Additional Details of the Scheme"
/// This is NOT ∇ itself!
pub(super) struct DeltaTable {
    /// Rows: 16 because we have 4 "bits": s00,s01,s10,s11
    rows: [delta_row::DeltaRow; 16],
    /// We use this field mostly as a "is_ready" flag
    /// It SHOULD be set by "step4_set_for_gate" to mark the table as ready for "compute_s1"
    gate_type: Option<GateType>,
}

impl DeltaTable {
    /// Build a new `DeltaTable` for the given `Gate`(or rather `GateType`).
    ///
    /// "B Garbling Other Gates"
    /// "(iii) With Table 1 as a template, initialize a new 16-row table T ,
    /// whose index is the vector [X00, X01, X10, X11] and its value is ∇. Initialize all ∇
    /// values to 0 (i.e., T [X00, X01, X10, X11] = 0 for all X00, X01, X10, and X11);"
    pub(super) fn new_for_gate(gate: &Gate) -> Self {
        /// this will be the vector of X00 X01 X10 X11
        /// 0000
        /// 0001
        /// 0010
        /// ...
        /// -> so 16 rows
        let mut delta_rows = Vec::with_capacity(16);
        for x00 in 0..2 {
            for x01 in 0..2 {
                for x10 in 0..2 {
                    for x11 in 0..2 {
                        // "!= 0" is just to convert integer -> bool
                        delta_rows.push(delta_row::DeltaRow::new(
                            x00 != 0,
                            x01 != 0,
                            x10 != 0,
                            x11 != 0,
                        ));
                    }
                }
            }
        }

        let mut delta_table = Self {
            rows: delta_rows.try_into().unwrap(),
            gate_type: None,
        };

        delta_table.step4_set_for_gate(gate);

        assert!(delta_table.is_ready());

        delta_table
    }

    fn is_ready(&self) -> bool {
        self.gate_type.is_some()
    }

    /// "(iv) Set ∇ = 1 in the rows indexed by the vectors from Step (ii), as well as the first and last rows."
    fn step4_set_for_gate(&mut self, gate: &Gate) {
        let truth_table = TruthTable::new_from_gate(gate.internal.get_type());

        // "Set ∇ = 1 in the rows indexed by the vectors from Step (ii)"
        for row in self.rows.iter_mut() {
            if row.get_Xab() == truth_table.truth_table
                || row.get_Xab() == truth_table.get_complement()
            {
                row.set_delta_true();
            }
        }
        // "as well as the first and last rows."
        self.rows[0].set_delta_true();
        self.rows[15].set_delta_true();

        self.gate_type = Some(gate.internal.get_type().clone());
    }

    /// Return the (x00,x01,x10,x11) values for which delta == 1
    /// eg for AND it will return {0000, 0001, 1110, 1111}
    /// and for XOR {0000, 1001, 0110, 1111}
    ///
    /// This is a helper for "Algorithm 5 Gate" line 8: and 10:
    fn get_delta_slices(&self) {
        self.rows.iter().filter(|row| row.get_delta())
    }

    // In the papers:
    // A ◦ B = projection of A[i] for positions with B[i] = 1
    // also noted & as in: S0 = X00 & ∇
    //
    // Return:
    // other_vec[i] for all positions of "self" where self[i] = 1
    // == other_vec & self
    // == other_vec ◦ self
    // fn project_x00_delta(&self) -> Vec<WireInternal> {
    //     self.rows
    //         .iter()
    //         .map(|delta_row: &delta_row::DeltaRow| delta_row.get_x00())
    //         .into_iter()
    //         .zip(self.rows.iter().map(|delta_row| delta_row.get_delta()))
    //         .map(|(x00, delta)| if delta { x00.clone() } else { false })
    //         .collect()
    // }

    // Compute "s1"
    // ie project the appropriate X0/X01/.. onto "delta" based on the current gate's truth table
    // or "delta table" in the paper
    //
    // IMPORTANT: "step4_set_for_gate" SHOULD have been called before this!
    // fn compute_s1(&self) -> Vec<WireInternal> {
    //     assert!(
    //         self.is_ready(),
    //         "compute_s1 MUST be called AFTER step4_set_for_gate!"
    //     );

    //     // "the right side demonstrates how combining Xij [j]&∇ collapses into only two distinct values"
    //     // Let's check!
    //     let counts = {
    //         let mut s00_col = vec![];
    //         let mut s01_col = vec![];
    //         let mut s10_col = vec![];
    //         let mut s11_col = vec![];
    //         for delta_row in self.rows.iter() {
    //             s00_col.push(delta_row.get_s00());
    //             s01_col.push(delta_row.get_s01());
    //             s10_col.push(delta_row.get_s10());
    //             s11_col.push(delta_row.get_s11());
    //         }
    //         let s_cols = vec![s00_col, s01_col, s10_col, s11_col];

    //         let mut map_counts: HashMap<u16, usize> = HashMap::new();
    //         let s_cols_u16: Vec<u16> = s_cols.iter().map(|s_col| vec_bool_to_u16(s_col)).collect();
    //         for s_val_u16 in s_cols_u16 {
    //             let mut values = map_counts.entry(s_val_u16).or_default();
    //             *values += 1;
    //         }
    //         println!("compute_s1: counts: {:?}", map_counts);
    //         // "only two distinct values"
    //         assert_eq!(map_counts.len(), 2, "SHOULD only have 2 distinct values!");

    //         map_counts
    //     };

    //     let gate_type = self.gate_type.as_ref().unwrap();
    //     match gate_type {
    //         // GateType::ZERO => todo!(),
    //         // GateType::INV => todo!(),
    //         GateType::XOR => todo!(),
    //         GateType::AND => todo!(),
    //         // GateType::ONE => todo!(),
    //     }
    //     todo!()
    // }
}

/// Represent the truth table for a 2 inputs boolean gate
/// ordered classically as: 00, 01, 10, 11
struct TruthTable {
    truth_table: [bool; 4],
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
            GateType::XOR => TruthTable {
                truth_table: [false, true, true, false],
            },
            // GateType::NAND => TruthTable {
            //     truth_table: [true, true, true, false],
            // },
            GateType::AND => TruthTable {
                truth_table: [false, false, false, true],
            },
            // GateType::XNOR => todo!(),
            // // TODO? BUF(A) = XOR(A, 0), BUF(A) = NOR(NOR(A, A), 0), BUF(A) = OR(A, 0), BUF(A) = NAND(A, NAND(A, 0)), BUF(A) = AND(A, 1)
            // GateType::BUF => todo!(),
            // GateType::AONB => todo!(),
            // GateType::BUFB => todo!(),
            // GateType::NAOB => todo!(),
            // GateType::OR => TruthTable {
            //     truth_table: [false, true, true, true],
            // },
            // TODO? NAND(A, 0) always outputs 1 since NAND outputs 0 only when both inputs are 1.
            // GateType::ONE => todo!(),
        }
    }

    pub(self) fn get_complement(&self) -> [bool; 4] {
        let mut complement: [bool; 4] = [false; 4];
        for (i, val) in self.truth_table.into_iter().enumerate() {
            complement[i] = !val;
        }
        complement
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::garble::new_garbling_scheme::*;
    use crate::garble::InterstellarCircuit;

    #[test]
    fn test_delta_Xab() {
        // NOTE: for this we only care about the first 4 cols; so the GateType does not matter
        let gate = Gate {
            internal: GateInternal::Standard {
                r#type: GateType::AND,
                input_a: None,
                input_b: None,
            },
            output: WireRef { id: 1 },
        };
        let delta_table = DeltaTable::new_for_gate(&gate);

        assert_eq!(delta_table.rows[0].get_Xab(), [false, false, false, false]);
        assert_eq!(delta_table.rows[1].get_Xab(), [false, false, false, true,]);
        assert_eq!(delta_table.rows[10].get_Xab(), [true, false, true, false,]);
        assert_eq!(delta_table.rows[15].get_Xab(), [true, true, true, true,]);
    }

    #[test]
    fn test_delta_AND() {
        let gate = Gate {
            internal: GateInternal::Standard {
                r#type: GateType::AND,
                input_a: None,
                input_b: None,
            },
            output: WireRef { id: 1 },
        };
        let mut delta_table = DeltaTable::new_for_gate(&gate);

        assert_eq!(delta_table.rows[0].get_delta(), true);
        assert_eq!(delta_table.rows[1].get_delta(), true);
        assert_eq!(delta_table.rows[14].get_delta(), true);
        assert_eq!(delta_table.rows[15].get_delta(), true);
    }

    #[test]
    fn test_delta_XOR() {
        let gate = Gate {
            internal: GateInternal::Standard {
                r#type: GateType::XOR,
                input_a: None,
                input_b: None,
            },
            output: WireRef { id: 1 },
        };
        let mut delta_table = DeltaTable::new_for_gate(&gate);

        assert_eq!(delta_table.rows[0].get_delta(), true);
        assert_eq!(delta_table.rows[6].get_delta(), true);
        assert_eq!(delta_table.rows[9].get_delta(), true);
        assert_eq!(delta_table.rows[15].get_delta(), true);
    }

    // #[test]
    // fn test_project_x00_delta_all_1() {
    //     let mut delta_table = DeltaTable::new_default();
    //     for delta_row in delta_table.rows.iter_mut() {
    //         delta_row.set_x00_delta(true, true);
    //     }

    //     let res = delta_table.project_x00_delta();

    //     assert!(res.iter().all(|&e| e));
    // }

    // #[test]
    // fn test_project_x00_delta_10() {
    //     let mut delta_table = DeltaTable::new_default();
    //     for delta_row in delta_table.rows.iter_mut() {
    //         delta_row.set_x00_delta(true, false);
    //     }

    //     let res = delta_table.project_x00_delta();

    //     assert!(res.iter().all(|&e| !e));
    // }

    // #[test]
    // fn test_project_x00_delta_01() {
    //     let mut delta_table = DeltaTable::new_default();
    //     for delta_row in delta_table.rows.iter_mut() {
    //         delta_row.set_x00_delta(false, true);
    //     }

    //     let res = delta_table.project_x00_delta();

    //     assert!(res.iter().all(|&e| !e));
    // }

    // #[test]
    // fn test_compute_s1() {
    //     let gate = Gate {
    //         internal: GateInternal::Standard {
    //             r#type: GateType::AND,
    //             input_a: None,
    //             input_b: None,
    //         },
    //         output: WireRef { id: 1 },
    //     };
    //     let mut delta_table = DeltaTable::new_for_gate(&gate);

    //     assert_eq!(delta_table.rows[0].get_delta(), true);
    //     assert_eq!(delta_table.rows[1].get_delta(), true);
    //     assert_eq!(delta_table.rows[14].get_delta(), true);
    //     assert_eq!(delta_table.rows[15].get_delta(), true);
    // }
}
