#[cfg(all(not(feature = "std"), feature = "sgx"))]
use sgx_tstd::vec::Vec;

use crate::garble::GarblerInput;
use image::{GrayImage, Luma};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use snafu::prelude::*;

const FONT_BYTES: &[u8] = include_bytes!("../examples/data/BF_Modernista-Regular.ttf");
const WATERMARK_COLOR: [u8; 1] = [255u8];

/// The given integer is NOT a valid 7 segments option[ie 0-9]
#[derive(Debug, Snafu)]
#[snafu(display("Can open read the .ttf"))]
pub(crate) struct FontTtfErr {}

/// Init a Font using the hardcoded .ttf from "data/"
fn new_font<'a>() -> Result<Font<'a>, FontTtfErr> {
    Font::try_from_bytes(FONT_BYTES).ok_or(FontTtfErr {})
}

/// imageproc's `draw_text_mut` DOES NOT support multiline so we need to handle it on our side
/// "this function does not support newlines, you must do this manually"
/// cf [imageproc docs](https://docs.rs/imageproc/latest/imageproc/drawing/fn.draw_text_mut.html)
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn my_draw_text_mut_with_newline(
    image: &mut GrayImage,
    color: Luma<u8>,
    x: i32,
    y: i32,
    scale: Scale,
    font: &Font<'_>,
    text: &str,
) {
    for (line_no, line_str) in text.lines().enumerate() {
        draw_text_mut(
            image,
            color,
            x,
            y + (scale.y as i32 * line_no as i32),
            scale,
            font,
            line_str,
        );
    }
}

/// Draw a basic text onto a new image
/// cf [imageproc examples](https://github.com/Interstellar-Network/imageproc/blob/master/examples/font.rs)
///
/// Return: a GRAYSCALE image; len = `img_height` * `img_width`
#[allow(clippy::cast_possible_wrap)]
fn my_draw_text_mut(image: &mut GrayImage, text: &str) -> Result<(), FontTtfErr> {
    let font = new_font()?;

    // TODO(interstellar) adjust pos and size; ideally measure the final text then center it as best as we can
    // eg use "text_size" etc
    let height = 40.4;
    let scale = Scale {
        x: height * 2.0,
        y: height,
    };
    let text_pos_x = image.width() as i32 / 4;
    let text_pos_y = image.height() as i32 / 2;

    my_draw_text_mut_with_newline(
        image,
        Luma(WATERMARK_COLOR),
        text_pos_x,
        text_pos_y,
        scale,
        &font,
        text,
    );

    Ok(())
}

/// "Convert" GrayImage(ie result of `draw_text` etc) to the correct input type for
/// garb.eval()
/// NOTE: `GrayImage` has pixels whose values is [0-255], but garb.eval() expects only [0-1]
/// so we convert them.
///
/// ie Vec<u8> -> Vec<u16>
/// This is NOT doing anything funny to the bits, no shuffling etc
/// It is just raw conversion result[i] = input[i]
fn convert_image_to_garbler_inputs(image: GrayImage) -> Vec<GarblerInput> {
    image
        .into_vec()
        .into_iter()
        .map(|pixel| {
            // IMPORTANT: we NEED a threshold here b/c "draw_text_mut" has apparently some AA
            u16::from(pixel > 0)
        })
        .collect()
}

/// NOTE: our use case is to create a "watermark", that's why we create(and discard) the image here
/// instead of passing it as parameter.
/// cf `convert_image_to_garbler_inputs`
pub(crate) fn new_watermark(
    img_width: u32,
    img_height: u32,
    text: &str,
) -> Result<Vec<GarblerInput>, FontTtfErr> {
    let mut image = GrayImage::new(img_width, img_height);

    my_draw_text_mut(&mut image, text)?;
    assert_eq!(
        image.len(),
        img_width as usize * img_height as usize,
        "watermark: wrong size!"
    );

    Ok(convert_image_to_garbler_inputs(image))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::EncodableLayout;
    use tests_utils::png_utils::read_png_to_bytes;

    #[test]
    fn test_convert_image_to_garbler_inputs_black_white() {
        let image = GrayImage::from_vec(4, 1, vec![255, 0, 0, 255]).unwrap();

        assert_eq!(convert_image_to_garbler_inputs(image), vec![1u16, 0, 0, 1]);
    }

    #[test]
    fn test_convert_image_to_garbler_inputs_grays() {
        let image = GrayImage::from_vec(4, 1, vec![128, 10, 0, 1]).unwrap();

        assert_eq!(convert_image_to_garbler_inputs(image), vec![1u16, 1, 0, 1]);
    }

    fn test_my_draw_text_mut(text: &str, expected_png_bytes: &[u8]) {
        let width = 600;
        let height = 200;
        let mut image = GrayImage::new(width, height);

        my_draw_text_mut(&mut image, text);

        let expected_png = read_png_to_bytes(expected_png_bytes);
        // WHEN UPDATING TEST:
        // tests_utils::png_utils::write_png_direct(
        //     "TOREMOVE.png",
        //     width as usize,
        //     height as usize,
        //     image.as_bytes(),
        // );
        assert_eq!(image.as_bytes(), expected_png);
    }

    #[test]
    fn test_draw_text_one_line_ascii() {
        test_my_draw_text_mut(
            "Hello world",
            include_bytes!("../examples/data/test_draw_text_one_line_ascii.png"),
        );
    }

    #[test]
    fn test_draw_text_two_lines_ascii_lf() {
        test_my_draw_text_mut(
            "Hello\nworld",
            include_bytes!("../examples/data/test_draw_text_two_lines_ascii.png"),
        );
    }

    #[test]
    fn test_draw_text_two_lines_ascii_crlf() {
        test_my_draw_text_mut(
            "Hello\r\nworld",
            include_bytes!("../examples/data/test_draw_text_two_lines_ascii.png"),
        );
    }

    #[test]
    fn test_draw_text_french() {
        test_my_draw_text_mut(
            "Héllô",
            include_bytes!("../examples/data/test_draw_text_french.png"),
        );
    }
}
