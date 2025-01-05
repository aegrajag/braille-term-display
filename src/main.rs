use std::cmp::min;

use clap::Parser;
use image::{
    imageops::{colorops::dither, ColorMap, FilterType},
    ImageBuffer, ImageReader, Rgb,
};
use term_size;

/// Renders an image in braille
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input image file path
    #[arg(required = true)]
    file_path: String,

    /// Render width (optional)
    #[arg(short, long)]
    width: Option<u32>,
}

struct CustomColorMap;

impl ColorMap for CustomColorMap {
    type Color = Rgb<u8>;

    fn index_of(&self, color: &Self::Color) -> usize {
        // Calculate luminance
        let luminance =
            (0.2126 * color[0] as f32 + 0.7152 * color[1] as f32 + 0.0722 * color[2] as f32) as u8;

        // Return 0 for black, 1 for white
        if luminance < 128 {
            0
        } else {
            1
        }
    }
    fn map_color(&self, color: &mut Self::Color) {
        let palette = vec![Rgb([0; 3]), Rgb([255; 3])];
        let index = self.index_of(color);
        *color = palette[index];
    }
}

// TODO fix this
fn pixels_to_braille(x: u32, y: u32, image: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> Option<char> {
    let mut addon = 0;
    for i in 0..2 {
        for j in 0..4 {
            if x + i < image.width() && y + j < image.height() {
                let [r, g, b] = image.get_pixel(x + i, y + j).0;
                if (r as u16 + g as u16 + b as u16) / 3 > 128 {
                    addon |= if j != 3 {
                        1 << (i * 3 + j)
                    } else {
                        1 << (6 + i)
                    };
                }
            }
        }
    }
    return char::from_u32(0x2800 + addon);
}

fn main() {
    let args = Args::parse(); // We get the args
    match ImageReader::open(args.file_path) {
        Ok(reader) => {
            let image = reader.decode().unwrap().to_rgb8();

            const DEFAULT_WIDTH: u32 = 120 * 2;
            let aspect_ratio = if image.width() != 0 {
                image.height() as f32 / image.width() as f32
            } else {
                0.
            };

            let new_width = if let Some(arg_width) = args.width {
                arg_width * 2
            } else {
                match term_size::dimensions() {
                    Some((term_w, term_h)) => {
                        let term_w_32 = u32::try_from(term_w * 2);
                        let term_h_32 = u32::try_from((term_h - 1) * 4);

                        match (term_w_32, term_h_32) {
                            (Ok(term_w_32), Ok(term_h_32)) => {
                                println!("{}", aspect_ratio);
                                let aspect_ratio_adjusted = if aspect_ratio != 0. {
                                    (term_h_32 as f32 / aspect_ratio) as u32
                                } else {
                                    DEFAULT_WIDTH
                                };
                                min(term_w_32, aspect_ratio_adjusted)
                            }
                            _ => DEFAULT_WIDTH,
                        }
                    }
                    None => DEFAULT_WIDTH,
                }
            };
            let new_height = (new_width as f32 * aspect_ratio) as u32;
            let mut resized_image =
                image::imageops::resize(&image, new_width, new_height, FilterType::Lanczos3);

            let colormap = CustomColorMap;

            dither(&mut resized_image, &colormap);

            for y in (0..new_height).step_by(4) {
                for x in (0..new_width).step_by(2) {
                    print!(
                        "{}",
                        pixels_to_braille(x, y, &resized_image)
                            .as_ref()
                            .unwrap_or(&' ')
                    );
                }
                println!("")
            }
        }
        Err(e) => {
            eprintln!("#Error opening the image: {}", e);
        }
    }
}
