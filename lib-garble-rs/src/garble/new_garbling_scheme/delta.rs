use crate::garble::new_garbling_scheme::{Gate, GateInternal, GateType, WireInternal};

/// Represent a ROW in the "delta table"
/// X00 X01 X10 X11 ∇ S00 S01 S10 S11
// TODO this is probably dup with Wire and/or Block
#[derive(Debug, PartialEq)]
struct DeltaRow {
    x00: WireInternal,
    x01: WireInternal,
    x10: WireInternal,
    x11: WireInternal,
    delta: WireInternal,
    s00: WireInternal,
    s01: WireInternal,
    s10: WireInternal,
    s11: WireInternal,
}

impl DeltaRow {
    pub(self) fn new(
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
            delta: false,
            s00: false,
            s01: false,
            s10: false,
            s11: false,
        }
    }

    pub(self) fn get_Xab(&self) -> [WireInternal; 4] {
        [self.x00, self.x01, self.x10, self.x11]
    }

    pub(self) fn get_delta(&self) -> WireInternal {
        self.delta
    }
}

pub(super) struct DeltaTable {
    pub(self) rows: [DeltaRow; 16],
}

impl DeltaTable {
    /// "B Garbling Other Gates"
    /// "(iii) With Table 1 as a template, initialize a new 16-row table T ,
    /// whose index is the vector [X00, X01, X10, X11] and its value is ∇. Initialize all ∇
    /// values to 0 (i.e., T [X00, X01, X10, X11] = 0 for all X00, X01, X10, and X11);"
    pub(super) fn new_default() -> Self {
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
                        delta_rows.push(DeltaRow::new(x00 != 0, x01 != 0, x10 != 0, x11 != 0));
                    }
                }
            }
        }

        Self {
            rows: delta_rows.try_into().unwrap(),
        }
    }

    /// "(iv) Set ∇ = 1 in the rows indexed by the vectors from Step (ii), as well as the first and last rows."
    fn step4_set_for_gate(&mut self, gate: &Gate) {
        let truth_table = TruthTable::new_from_gate(gate.internal.get_type());

        // "Set ∇ = 1 in the rows indexed by the vectors from Step (ii)"
        for row in self.rows.iter_mut() {
            if row.get_Xab() == truth_table.truth_table
                || row.get_Xab() == truth_table.get_complement()
            {
                row.delta = true;
            }
        }
        // "as well as the first and last rows."
        self.rows[0].delta = true;
        self.rows[15].delta = true;
    }
}

/// Represent the truth table for a 2 inputs boolean gate
/// ordered classically as: 00, 01, 10, 11
struct TruthTable {
    truth_table: [bool; 4],
}

impl TruthTable {
    pub(self) fn new_from_gate(gate_type: &GateType) -> Self {
        match gate_type {
            GateType::ZERO => todo!(),
            GateType::NOR => TruthTable {
                truth_table: [true, false, false, false],
            },
            GateType::AANB => todo!(),
            GateType::INVB => todo!(),
            GateType::NAAB => todo!(),
            GateType::INV => todo!(),
            GateType::XOR => TruthTable {
                truth_table: [false, true, true, false],
            },
            GateType::NAND => TruthTable {
                truth_table: [true, true, true, false],
            },
            GateType::AND => TruthTable {
                truth_table: [false, false, false, true],
            },
            GateType::XNOR => todo!(),
            GateType::BUF => todo!(),
            GateType::AONB => todo!(),
            GateType::BUFB => todo!(),
            GateType::NAOB => todo!(),
            GateType::OR => TruthTable {
                truth_table: [false, true, true, true],
            },
            GateType::ONE => todo!(),
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
    fn test_generate_default_delta_table() {
        let delta_table = DeltaTable::new_default();

        assert_eq!(delta_table.rows[0].get_Xab(), [false, false, false, false]);
        assert_eq!(delta_table.rows[1].get_Xab(), [false, false, false, true,]);
        assert_eq!(delta_table.rows[10].get_Xab(), [true, false, true, false,]);
        assert_eq!(delta_table.rows[15].get_Xab(), [true, true, true, true,]);
    }

    #[test]
    fn test_delta_OR() {
        let mut delta_table = DeltaTable::new_default();
        let gate = Gate {
            internal: GateInternal::Standard {
                r#type: GateType::AND,
                input_a: None,
                input_b: None,
            },
            output: WireRef { id: 1 },
        };
        delta_table.step4_set_for_gate(&gate);

        assert_eq!(delta_table.rows[0].get_delta(), true);
        assert_eq!(delta_table.rows[1].get_delta(), true);
        assert_eq!(delta_table.rows[14].get_delta(), true);
        assert_eq!(delta_table.rows[15].get_delta(), true);
    }

    #[test]
    fn test_delta_XOR() {
        let mut delta_table = DeltaTable::new_default();
        let gate = Gate {
            internal: GateInternal::Standard {
                r#type: GateType::XOR,
                input_a: None,
                input_b: None,
            },
            output: WireRef { id: 1 },
        };
        delta_table.step4_set_for_gate(&gate);

        assert_eq!(delta_table.rows[0].get_delta(), true);
        assert_eq!(delta_table.rows[6].get_delta(), true);
        assert_eq!(delta_table.rows[9].get_delta(), true);
        assert_eq!(delta_table.rows[15].get_delta(), true);
    }
}
