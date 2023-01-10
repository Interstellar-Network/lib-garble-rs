use alloc::vec::Vec;
use num_enum::TryFromPrimitive;
use snafu::prelude::*;

use crate::garble::GarblerInput;

#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
enum SegmentsSevenKind {
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
}

/// The given integer is NOT a valid 7 segments option[ie 0-9]
#[derive(Debug, Snafu)]
#[snafu(display("Can not convert number {} to SegmentsSevenKind", number))]
pub(crate) struct SegmentsError {
    pub(crate) number: u8,
}

/// Used when preparing the watermark
/// Convert eg [4,2] ->
///  first digit: 7 segments: 4
/// 0u16, 1, 1, 1, 0, 1, 0, //
/// // second digit: 7 segments: 2
/// 1u16, 0, 1, 1, 1, 0, 1, //
pub(crate) fn digits_to_segments_bits(digits: &[u8]) -> Result<Vec<GarblerInput>, SegmentsError> {
    // 7 BITS per digit input
    let mut res = Vec::with_capacity(digits.len() * 7);

    for digit in digits {
        match SegmentsSevenKind::try_from(*digit).map_err(|e| SegmentsError { number: e.number })? {
            SegmentsSevenKind::Zero => todo!(),
            SegmentsSevenKind::One => todo!(),
            SegmentsSevenKind::Two => res.extend_from_slice(&[1u16, 0, 1, 1, 1, 0, 1]),
            SegmentsSevenKind::Three => todo!(),
            SegmentsSevenKind::Four => res.extend_from_slice(&[0u16, 1, 1, 1, 0, 1, 0]),
            SegmentsSevenKind::Five => todo!(),
            SegmentsSevenKind::Six => todo!(),
            SegmentsSevenKind::Seven => todo!(),
            SegmentsSevenKind::Eight => todo!(),
            SegmentsSevenKind::Nine => todo!(),
        }
    }

    Ok(res)
}
