use image::{DynamicImage, FilterType, GenericImage, Pixel, Rgba, RgbaImage};
use rusttype::{point, Font, Scale};

use config::{BORDER_SIZE, FONT_SIZE, OUTER_MARGIN, TEXT_MARGIN};

fn render_text(font: &Font, text: &str) -> RgbaImage {
    let scale = Scale::uniform(FONT_SIZE);
    let v_metrics = font.v_metrics(scale);
    let glyphs: Vec<_> = font
        .layout(
            text,
            scale,
            point(TEXT_MARGIN, TEXT_MARGIN + v_metrics.ascent),
        ).collect();

    let glyphs_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
    let glyphs_width = {
        let min_x = glyphs
            .first()
            .map(|g| g.pixel_bounding_box().unwrap().min.x)
            .unwrap();
        let max_x = glyphs
            .last()
            .map(|g| g.pixel_bounding_box().unwrap().max.x)
            .unwrap();
        (max_x - min_x) as u32
    };

    let mut im = DynamicImage::new_rgba8(glyphs_width + 40, glyphs_height + 40).to_rgba();
    // FIXME: This is probably unnecessary?
    for pixel in im.pixels_mut() {
        *pixel = Rgba { data: [0, 0, 0, 0] }
    }

    for glyph in &glyphs {
        if let Some(bounds) = glyph.pixel_bounding_box() {
            glyph.draw(|px, py, v| {
                for xoff in 0..=(BORDER_SIZE * 2) {
                    for yoff in 0..=(BORDER_SIZE * 2) {
                        let x = px + bounds.min.x as u32 + xoff - BORDER_SIZE;
                        let y = py + bounds.min.y as u32 + yoff - BORDER_SIZE;
                        let old_p = im.get_pixel_mut(x, y);
                        let new_p = Rgba {
                            data: [0, 0, 0, (v * 255.0) as u8],
                        };
                        old_p.blend(&new_p);
                    }
                }
            });
        }
    }
    for glyph in &glyphs {
        if let Some(bounds) = glyph.pixel_bounding_box() {
            glyph.draw(|px, py, v| {
                let x = px + bounds.min.x as u32;
                let y = py + bounds.min.y as u32;
                let old_p = im.get_pixel_mut(x, y);
                let new_p = Rgba {
                    data: [255, 255, 255, (v * 255.0) as u8],
                };
                old_p.blend(&new_p);
            });
        }
    }

    im
}

fn blend_copy(im1: &mut RgbaImage, im2: &RgbaImage, target_x: u32, target_y: u32) {
    for (x, y, p2) in im2.enumerate_pixels() {
        let p1 = im1.get_pixel_mut(target_x + x, target_y + y);
        p1.blend(p2);
    }
}

fn adjust_target(im: RgbaImage, target_width: u32) -> RgbaImage {
    if im.width() < target_width {
        let mut out = DynamicImage::new_rgba8(target_width, im.height()).to_rgba();
        out.copy_from(&im, (target_width - im.width()) / 2, 0);
        out
    } else if im.width() > target_width {
        let dim = DynamicImage::ImageRgba8(im);
        dim.resize(target_width, dim.height(), FilterType::CatmullRom)
            .to_rgba()
    } else {
        im
    }
}

pub fn draw_on_image(
    im: &DynamicImage,
    font: &Font,
    top_text: &str,
    bottom_text: &str,
) -> DynamicImage {
    let top = adjust_target(render_text(font, top_text), im.width());
    let bottom = adjust_target(render_text(font, bottom_text), im.width());
    let mut buf = im.to_rgba();
    blend_copy(&mut buf, &top, 0, OUTER_MARGIN);
    blend_copy(
        &mut buf,
        &bottom,
        0,
        im.height() - bottom.height() - OUTER_MARGIN,
    );
    DynamicImage::ImageRgba8(buf)
}
