/// cf https://docs.rs/png/latest/png/#using-the-decoder
pub fn read_png_to_bytes(buf: &[u8]) -> Vec<u8> {
    // The decoder is a build for reader and can be used to set various decoding options
    // via `Transformations`. The default output transformation is `Transformations::IDENTITY`.
    let decoder = png::Decoder::new(buf);
    let mut reader = decoder.read_info().unwrap();
    // Allocate the output buffer.
    let mut buf = vec![0; reader.output_buffer_size()];
    // Read the next frame. An APNG might contain multiple frames.
    let info = reader.next_frame(&mut buf).unwrap();
    // Grab the bytes of the image.
    let bytes = &buf[..info.buffer_size()];

    bytes.to_vec()
}

pub fn convert_vec_u16_to_u8(data: &[u16]) -> Vec<u8> {
    let data: Vec<u8> = data
        .iter()
        .map(|v| {
            let pixel_value: u8 = (*v).try_into().unwrap();
            pixel_value * 255
        })
        .collect();

    data
}

/// param outputs: result of garb.eval()
/// return: the raw bytes of .png corresponding to the GarbledCircuit's eval outputs
/// Typically the is "output[i] = eval[i] * 255"
pub fn write_png(path: &str, width: usize, height: usize, data: &[u16]) {
    let data_u8 = convert_vec_u16_to_u8(data);

    write_png_direct(path, width, height, &data_u8);
}

pub fn write_png_direct(path: &str, width: usize, height: usize, data: &[u8]) {
    use std::io::BufWriter;

    // use std::io::Cursor;
    // let buf = Vec::new();
    // let c = Cursor::new(buf);
    // let ref mut w = BufWriter::new(c);

    let file = std::fs::File::create(path).unwrap();
    let w = BufWriter::new(file);

    // TODO(interstellar) get from Circuit's "config"
    let mut encoder = png::Encoder::new(w, width.try_into().unwrap(), height.try_into().unwrap());
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().unwrap();

    writer.write_image_data(data).unwrap();

    writer.finish().unwrap();
}