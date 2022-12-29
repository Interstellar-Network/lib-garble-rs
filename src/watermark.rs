#[cfg(all(not(feature = "std"), feature = "sgx"))]
use sgx_tstd::vec::Vec;

use crate::garble::EvaluatorInput;
use image::{GrayImage, Luma};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};

const FONT_BYTES: &[u8] = include_bytes!("../examples/data/BF_Modernista-Regular.ttf");
const WATERMARK_COLOR: [u8; 1] = [255u8];

/// Init a Font using the hardcoded .ttf from "data/"
pub fn new_font<'a>() -> Font<'a> {
    Font::try_from_bytes(FONT_BYTES).unwrap()
}

/// cf https://docs.rs/imageproc/latest/imageproc/drawing/fn.draw_text_mut.html
/// "this function does not support newlines, you must do this manually"
fn draw_text_mut_with_newline(
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
        )
    }
}

/// Draw a basic text onto an image
/// cf https://github.com/Interstellar-Network/imageproc/blob/master/examples/font.rs
///
/// Return: a GRAYSCALE image; len = img_height * img_width
pub fn draw_text(img_width: u32, img_height: u32, font: &Font<'_>, text: &str) -> GrayImage {
    let mut image = GrayImage::new(img_width, img_height);

    let height = 40.4;
    let scale = Scale {
        x: height * 2.0,
        y: height,
    };

    draw_text_mut_with_newline(
        &mut image,
        Luma(WATERMARK_COLOR),
        (img_width / 4).try_into().unwrap(),
        (img_height / 2).try_into().unwrap(),
        scale,
        font,
        text,
    );
    // TODO(interstellar)???
    // let (w, h) = text_size(scale, &font, text);
    // println!("Text size: {}x{}", w, h);

    assert_eq!(
        image.len(),
        img_width as usize * img_height as usize,
        "watermark: wrong size!"
    );
    image
}

/// "Convert" GrayImage(ie result of draw_text etc) to the correct input type for
/// garb.eval()
/// NOTE: "GrayImage" has pixels whose values is [0-255], but garb.eval() expects only [0-1]
/// so we convert them.
///
/// ie Vec<u8> -> Vec<u16>
/// This is NOT doing anything funny to the bits, no shuffling etc
/// It is just raw conversion result[i] = input[i]
pub fn convert_image_to_garbler_inputs(image: GrayImage) -> Vec<EvaluatorInput> {
    image
        .into_vec()
        .into_iter()
        .map(|pixel| {
            // IMPORTANT: we NEED a threshold here b/c "draw_text_mut" has apparently some AA
            let pixel = i32::from(pixel > 0);
            pixel.try_into().unwrap()
        })
        .collect()
}
