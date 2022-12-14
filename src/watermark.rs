#[cfg(all(not(feature = "std"), feature = "sgx"))]
use sgx_tstd::vec::Vec;

use image::ImageBuffer;
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_text_mut, text_size};
use rusttype::{Font, Scale};

/// Draw a basic text onto an image
/// cf https://github.com/Interstellar-Network/imageproc/blob/master/examples/font.rs
pub fn draw_text(img_width: u32, img_height: u32) -> ImageBuffer<image::Rgb<u8>, Vec<u8>> {
    let mut image = RgbImage::new(img_width, img_height);

    let font = Vec::from(include_bytes!("../examples/data/BF_Modernista-Regular.ttf") as &[u8]);
    let font = Font::try_from_vec(font).unwrap();

    let height = 12.4;
    let scale = Scale {
        x: height * 2.0,
        y: height,
    };

    let text = "Hello, world!";
    draw_text_mut(&mut image, Rgb([0u8, 0u8, 255u8]), 0, 0, scale, &font, text);
    // TODO(interstellar)???
    // let (w, h) = text_size(scale, &font, text);
    // println!("Text size: {}x{}", w, h);

    image
}
