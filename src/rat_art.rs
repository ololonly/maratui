// Pixel-perfect rat chef drawing using embedded-graphics
use embedded_graphics::{
    Drawable, geometry::Point, pixelcolor::Bgr565, prelude::*, primitives::Rectangle,
};

#[cfg(feature = "preview")]
use image;

/// Load and draw image from file (for preview mode)
/// Returns true on success, false on error
#[cfg(feature = "preview")]
pub fn draw_image_from_file<D>(target: &mut D, file_path: &str, position: Point) -> bool
where
    D: DrawTarget<Color = Bgr565>,
{
    // Try multiple paths: relative, absolute from current dir, and from project root
    let paths_to_try = vec![
        file_path.to_string(),
        format!("./{}", file_path),
        format!("../{}", file_path),
    ];

    let img = {
        let mut loaded = None;
        for path in &paths_to_try {
            match image::open(path) {
                Ok(img) => {
                    loaded = Some(img);
                    break;
                }
                Err(e) => {
                    eprintln!("Failed to load image from {}: {}", path, e);
                }
            }
        }
        match loaded {
            Some(img) => img,
            None => return false,
        }
    };

    // Get target size (screen is 240x135, but we want to fit in left panel ~144px wide)
    let max_width = 100u32;
    let max_height = 100u32;

    // Scale image to fit
    let rgba_img = img.to_rgba8();
    let (orig_width, orig_height) = rgba_img.dimensions();

    // Calculate scale factor to fit in max_width x max_height
    let scale_x = max_width as f32 / orig_width as f32;
    let scale_y = max_height as f32 / orig_height as f32;
    let scale = scale_x.min(scale_y).min(1.0); // Don't upscale, only downscale

    let scaled_width = (orig_width as f32 * scale) as u32;
    let scaled_height = (orig_height as f32 * scale) as u32;

    let mut pixels_drawn = 0;
    for y in 0..scaled_height {
        for x in 0..scaled_width {
            // Sample from original image
            let orig_x = (x as f32 / scale) as u32;
            let orig_y = (y as f32 / scale) as u32;

            if orig_x >= orig_width || orig_y >= orig_height {
                continue;
            }

            let pixel = rgba_img.get_pixel(orig_x, orig_y);
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];
            let a = pixel[3];

            // Skip transparent pixels
            if a < 128 {
                continue;
            }

            // Convert RGB to Bgr565
            let color = Bgr565::new(
                (b as u16 * 31 / 255) as u8,
                (g as u16 * 63 / 255) as u8,
                (r as u16 * 31 / 255) as u8,
            );

            let draw_x = position.x + x as i32;
            let draw_y = position.y + y as i32;

            // Check bounds
            if draw_x < 0 || draw_y < 0 || draw_x >= 240 || draw_y >= 135 {
                continue;
            }

            if draw_pixel(target, Point::new(draw_x, draw_y), color).is_err() {
                eprintln!("Failed to draw pixel at ({}, {})", draw_x, draw_y);
                return false;
            }
            pixels_drawn += 1;
        }
    }

    true
}

pub fn draw_rat_chef<D>(target: &mut D, position: Point, frame: usize) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Bgr565>,
{
    let (x, y) = (position.x, position.y);

    // Colors
    let white = Bgr565::WHITE;
    let black = Bgr565::BLACK;
    let dark_gray = Bgr565::new(8, 8, 8); // Dark grey for coffee machine

    // === PIXEL-BY-PIXEL COPY OF REFERENCE ===
    // Position: x,y is center of head/body area
    // Drawing irregular, angular shapes exactly like reference

    let base_x = x - 15; // Offset to center the figure
    let base_y = y - 25; // Start from top

    // === TALL CHEF HAT (irregular, rounded top) ===
    // Top rounded part
    draw_pixel(target, Point::new(base_x + 4, base_y), white)?;
    draw_pixel(target, Point::new(base_x + 5, base_y), white)?;
    draw_pixel(target, Point::new(base_x + 6, base_y), white)?;
    draw_pixel(target, Point::new(base_x + 3, base_y + 1), white)?;
    draw_pixel(target, Point::new(base_x + 4, base_y + 1), white)?;
    draw_pixel(target, Point::new(base_x + 5, base_y + 1), white)?;
    draw_pixel(target, Point::new(base_x + 6, base_y + 1), white)?;
    draw_pixel(target, Point::new(base_x + 7, base_y + 1), white)?;
    draw_pixel(target, Point::new(base_x + 2, base_y + 2), white)?;
    draw_pixel(target, Point::new(base_x + 3, base_y + 2), white)?;
    draw_pixel(target, Point::new(base_x + 4, base_y + 2), white)?;
    draw_pixel(target, Point::new(base_x + 5, base_y + 2), white)?;
    draw_pixel(target, Point::new(base_x + 6, base_y + 2), white)?;
    draw_pixel(target, Point::new(base_x + 7, base_y + 2), white)?;
    draw_pixel(target, Point::new(base_x + 8, base_y + 2), white)?;
    // Hat body (irregular, extends down)
    draw_pixel(target, Point::new(base_x + 1, base_y + 3), white)?;
    draw_pixel(target, Point::new(base_x + 2, base_y + 3), white)?;
    draw_pixel(target, Point::new(base_x + 3, base_y + 3), white)?;
    draw_pixel(target, Point::new(base_x + 4, base_y + 3), white)?;
    draw_pixel(target, Point::new(base_x + 5, base_y + 3), white)?;
    draw_pixel(target, Point::new(base_x + 6, base_y + 3), white)?;
    draw_pixel(target, Point::new(base_x + 7, base_y + 3), white)?;
    draw_pixel(target, Point::new(base_x + 8, base_y + 3), white)?;
    draw_pixel(target, Point::new(base_x + 9, base_y + 3), white)?;
    draw_pixel(target, Point::new(base_x + 2, base_y + 4), white)?;
    draw_pixel(target, Point::new(base_x + 3, base_y + 4), white)?;
    draw_pixel(target, Point::new(base_x + 4, base_y + 4), white)?;
    draw_pixel(target, Point::new(base_x + 5, base_y + 4), white)?;
    draw_pixel(target, Point::new(base_x + 6, base_y + 4), white)?;
    draw_pixel(target, Point::new(base_x + 7, base_y + 4), white)?;
    draw_pixel(target, Point::new(base_x + 8, base_y + 4), white)?;
    draw_pixel(target, Point::new(base_x + 3, base_y + 5), white)?;
    draw_pixel(target, Point::new(base_x + 4, base_y + 5), white)?;
    draw_pixel(target, Point::new(base_x + 5, base_y + 5), white)?;
    draw_pixel(target, Point::new(base_x + 6, base_y + 5), white)?;
    draw_pixel(target, Point::new(base_x + 7, base_y + 5), white)?;

    // === COMPACT ROUNDED HEAD/BODY (below hat) ===
    draw_pixel(target, Point::new(base_x + 2, base_y + 6), white)?;
    draw_pixel(target, Point::new(base_x + 3, base_y + 6), white)?;
    draw_pixel(target, Point::new(base_x + 4, base_y + 6), white)?;
    draw_pixel(target, Point::new(base_x + 5, base_y + 6), white)?;
    draw_pixel(target, Point::new(base_x + 6, base_y + 6), white)?;
    draw_pixel(target, Point::new(base_x + 7, base_y + 6), white)?;
    draw_pixel(target, Point::new(base_x + 8, base_y + 6), white)?;
    draw_pixel(target, Point::new(base_x + 1, base_y + 7), white)?;
    draw_pixel(target, Point::new(base_x + 2, base_y + 7), white)?;
    draw_pixel(target, Point::new(base_x + 3, base_y + 7), white)?;
    draw_pixel(target, Point::new(base_x + 4, base_y + 7), white)?;
    draw_pixel(target, Point::new(base_x + 5, base_y + 7), white)?;
    draw_pixel(target, Point::new(base_x + 6, base_y + 7), white)?;
    draw_pixel(target, Point::new(base_x + 7, base_y + 7), white)?;
    draw_pixel(target, Point::new(base_x + 8, base_y + 7), white)?;
    draw_pixel(target, Point::new(base_x + 9, base_y + 7), white)?;
    draw_pixel(target, Point::new(base_x + 2, base_y + 8), white)?;
    draw_pixel(target, Point::new(base_x + 3, base_y + 8), white)?;
    draw_pixel(target, Point::new(base_x + 4, base_y + 8), white)?;
    draw_pixel(target, Point::new(base_x + 5, base_y + 8), white)?;
    draw_pixel(target, Point::new(base_x + 6, base_y + 8), white)?;
    draw_pixel(target, Point::new(base_x + 7, base_y + 8), white)?;
    draw_pixel(target, Point::new(base_x + 8, base_y + 8), white)?;

    // === SINGLE BLACK PIXEL EYE (slightly right and centered) ===
    draw_pixel(target, Point::new(base_x + 7, base_y + 7), black)?;

    // === ANGLED ARM (right side, pointing down-right) ===
    draw_pixel(target, Point::new(base_x + 10, base_y + 7), white)?;
    draw_pixel(target, Point::new(base_x + 11, base_y + 7), white)?;
    draw_pixel(target, Point::new(base_x + 12, base_y + 8), white)?;
    draw_pixel(target, Point::new(base_x + 13, base_y + 8), white)?;
    draw_pixel(target, Point::new(base_x + 14, base_y + 9), white)?;
    draw_pixel(target, Point::new(base_x + 15, base_y + 9), white)?;
    draw_pixel(target, Point::new(base_x + 16, base_y + 10), white)?;
    draw_pixel(target, Point::new(base_x + 17, base_y + 10), white)?;

    // === TILTED DARK GREY RECTANGLE (coffee machine, bottom-left, angled) ===
    let machine_base_x = base_x - 2;
    let machine_base_y = base_y + 12;

    // Draw tilted rectangle outline (white border)
    // Top edge (angled)
    draw_pixel(
        target,
        Point::new(machine_base_x + 1, machine_base_y),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 2, machine_base_y),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 3, machine_base_y),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 4, machine_base_y),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 5, machine_base_y),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 6, machine_base_y),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 7, machine_base_y),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 8, machine_base_y),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 9, machine_base_y),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 10, machine_base_y),
        white,
    )?;
    // Left edge
    draw_pixel(
        target,
        Point::new(machine_base_x, machine_base_y + 1),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x, machine_base_y + 2),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x, machine_base_y + 3),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x, machine_base_y + 4),
        white,
    )?;
    // Bottom edge (angled)
    draw_pixel(
        target,
        Point::new(machine_base_x + 1, machine_base_y + 5),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 2, machine_base_y + 5),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 3, machine_base_y + 5),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 4, machine_base_y + 5),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 5, machine_base_y + 5),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 6, machine_base_y + 5),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 7, machine_base_y + 5),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 8, machine_base_y + 5),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 9, machine_base_y + 5),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 10, machine_base_y + 5),
        white,
    )?;
    // Right edge
    draw_pixel(
        target,
        Point::new(machine_base_x + 10, machine_base_y + 1),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 10, machine_base_y + 2),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 10, machine_base_y + 3),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 10, machine_base_y + 4),
        white,
    )?;

    // Fill with dark grey
    Rectangle::new(
        Point::new(machine_base_x + 1, machine_base_y + 1),
        Size::new(9, 4),
    )
    .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(
        dark_gray,
    ))
    .draw(target)?;

    // White arrow pointing down (top-left inside)
    draw_pixel(
        target,
        Point::new(machine_base_x + 2, machine_base_y + 1),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 3, machine_base_y + 1),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 2, machine_base_y + 2),
        white,
    )?;

    // Horizontal white dash (below arrow, slightly right)
    draw_pixel(
        target,
        Point::new(machine_base_x + 5, machine_base_y + 3),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 6, machine_base_y + 3),
        white,
    )?;
    draw_pixel(
        target,
        Point::new(machine_base_x + 7, machine_base_y + 3),
        white,
    )?;

    Ok(())
}

fn draw_pixel<D>(target: &mut D, point: Point, color: Bgr565) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Bgr565>,
{
    Rectangle::new(point, Size::new(1, 1))
        .into_styled(embedded_graphics::primitives::PrimitiveStyle::with_fill(
            color,
        ))
        .draw(target)
}
