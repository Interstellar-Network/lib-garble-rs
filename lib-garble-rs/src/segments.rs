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

/// cf [`lib_garble`](https://github.com/Interstellar-Network/lib_garble/blob/main/src/packmsg/packmsg_utils.cpp#L26)
///
#[rustfmt::skip]
const MAP_DIGIT_TO7_SEGS: &[&[GarblerInput]] = &[
    // 0: all ON, except middle one(horizontal)
    &[   1,
      1, 1,
        0,
      1, 1,
        1
    ],
    // 1: only the 2 rightmost segments
    &[   0,
      0, 1,
        0,
      0, 1,
        0
    ],
    // 2
    &[   1,
      0, 1,
        1,
      1, 0,
        1
    ],
    // 3
    &[   1,
      0, 1,
        1,
      0, 1,
        1
    ],
    // 4
    &[   0,
      1, 1,
        1,
      0, 1,
        0
    ],
    // 5
    &[   1,
      1, 0,
        1,
      0, 1,
        1
    ],
    // 6
    &[   1,
      1, 0,
        1,
      1, 1,
        1
    ],
    // 7
    &[   1,
      0, 1,
        0,
      0, 1,
        0
    ],
    // 8
    &[   1,
      1, 1,
        1,
      1, 1,
        1
    ],
    // 9
    &[   1,
      1, 1,
        1,
      0, 1,
        1
    ]
];

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
        // let seven_seg =
        SegmentsSevenKind::try_from(*digit).map_err(|e| SegmentsError { number: e.number })?;
        // NOTE: if we are here, we know digit is a valid SegmentsSevenKind; but we DO NOT need its value
        // (ie we can re-use `*digit` instead)
        res.extend_from_slice(MAP_DIGIT_TO7_SEGS[*digit as usize]);
    }

    Ok(res)
}
