// Pixel-perfect rat chef drawing using embedded-graphics
use embedded_graphics::{
    Drawable, geometry::Point, pixelcolor::Bgr565, prelude::*, primitives::Rectangle,
};

use image::{self, AnimationDecoder, codecs::gif::GifDecoder};
use std::io::BufReader;
use std::time::{Duration, Instant};

/// Load and draw image from file (for preview mode)
/// Returns true on success, false on error
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
    let max_width = 140u32;
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
        }
    }

    true
}

/// Structure to hold GIF animation frames and timing
pub struct GifAnimation {
    frames: Vec<image::RgbaImage>,
    delays: Vec<Duration>,
    current_frame: usize,
    last_update: Instant,
    accumulated_time: Duration,
}

impl GifAnimation {
    /// Create a new empty animation
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            delays: Vec::new(),
            current_frame: 0,
            last_update: Instant::now(),
            accumulated_time: Duration::ZERO,
        }
    }

    /// Load GIF animation from file
    /// Returns true on success, false on error
    pub fn load_from_file(file_path: &str) -> Result<Self, String> {
        // Try multiple paths
        let paths_to_try = vec![
            file_path.to_string(),
            format!("./{}", file_path),
            format!("../{}", file_path),
        ];

        let file = {
            let mut found = None;
            for path in &paths_to_try {
                match std::fs::File::open(path) {
                    Ok(f) => {
                        found = Some(f);
                        break;
                    }
                    Err(e) => {
                        eprintln!("Failed to open GIF file {}: {}", path, e);
                    }
                }
            }
            found.ok_or_else(|| format!("Could not find GIF file: {}", file_path))?
        };

        let decoder = GifDecoder::new(BufReader::new(file))
            .map_err(|e| format!("Failed to create GIF decoder: {}", e))?;

        let mut frames = Vec::new();
        let mut delays = Vec::new();

        for frame_result in decoder.into_frames() {
            let frame = frame_result.map_err(|e| format!("Failed to decode frame: {}", e))?;
            let delay = frame.delay().into();
            let buffer = frame.into_buffer();
            frames.push(buffer);
            delays.push(delay);
        }

        if frames.is_empty() {
            return Err("GIF file contains no frames".to_string());
        }

        Ok(Self {
            frames,
            delays,
            current_frame: 0,
            last_update: Instant::now(),
            accumulated_time: Duration::ZERO,
        })
    }

    /// Update animation state based on current time
    /// Returns true if frame changed
    pub fn update(&mut self, now: Instant) -> bool {
        if self.frames.is_empty() {
            return false;
        }

        let elapsed = now.duration_since(self.last_update);
        self.last_update = now;
        self.accumulated_time += elapsed;

        // Check if we need to advance to next frame
        let current_delay = self
            .delays
            .get(self.current_frame)
            .copied()
            .unwrap_or(Duration::from_millis(100));

        if self.accumulated_time >= current_delay {
            self.accumulated_time -= current_delay;
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            return true;
        }

        false
    }

    /// Get current frame
    pub fn current_frame(&self) -> Option<&image::RgbaImage> {
        self.frames.get(self.current_frame)
    }

    /// Check if animation is loaded
    pub fn is_loaded(&self) -> bool {
        !self.frames.is_empty()
    }
}

impl Default for GifAnimation {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw current frame of GIF animation
pub fn draw_gif_frame<D>(
    target: &mut D,
    animation: &mut GifAnimation,
    position: Point,
    now: Instant,
) -> bool
where
    D: DrawTarget<Color = Bgr565>,
{
    // Update animation
    animation.update(now);

    let rgba_img = match animation.current_frame() {
        Some(img) => img,
        None => return false,
    };

    // Get target size
    let max_width = 140u32;
    let max_height = 100u32;

    let (orig_width, orig_height) = rgba_img.dimensions();

    // Calculate scale factor to fit in max_width x max_height
    let scale_x = max_width as f32 / orig_width as f32;
    let scale_y = max_height as f32 / orig_height as f32;
    let scale = scale_x.min(scale_y).min(1.0); // Don't upscale, only downscale

    let scaled_width = (orig_width as f32 * scale) as u32;
    let scaled_height = (orig_height as f32 * scale) as u32;

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
        }
    }

    true
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
