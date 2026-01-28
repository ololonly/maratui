// use mousefood::embedded_graphics::pixelcolor::{Rgb888, RgbColor};
// use mousefood::embedded_graphics::prelude::OriginDimensions;
// use mousefood::prelude::*;
// use mousefood::ratatui::buffer::Buffer;
// use mousefood::ratatui::layout::Rect;
// use mousefood::ratatui::widgets::Widget;
// use tinyqoi::Qoi;

// /// A widget that renders a QOI image using character blocks.
// /// Each character cell represents a 2x1 pixel block using half-block characters.
// pub struct QoiImage<'a> {
//     qoi: &'a Qoi<'a>,
// }

// impl<'a> QoiImage<'a> {
//     pub fn new(qoi: &'a Qoi<'a>) -> Self {
//         Self { qoi }
//     }
// }

// impl Widget for QoiImage<'_> {
//     fn render(self, area: Rect, buf: &mut Buffer) {
//         let size = self.qoi.size();
//         let width = size.width as usize;
//         let height = size.height as usize;

//         // Decode QOI to RGB pixels
//         let pixels: Vec<Rgb888> = self.qoi.pixels().collect();

//         // Each terminal cell represents 2 vertical pixels using half-block characters
//         let char_width = area.width as usize;
//         let char_height = area.height as usize;

//         for cy in 0..char_height {
//             for cx in 0..char_width {
//                 let x = area.x as usize + cx;
//                 let y = area.y as usize + cy;

//                 if x >= buf.area().width as usize || y >= buf.area().height as usize {
//                     continue;
//                 }

//                 // Map character position to pixel position
//                 // Scale image to fit the area
//                 let px = (cx * width) / char_width.max(1);
//                 let py_top = (cy * 2 * height) / (char_height * 2).max(1);
//                 let py_bottom = ((cy * 2 + 1) * height) / (char_height * 2).max(1);

//                 let top_pixel = if py_top < height && px < width {
//                     Some(pixels[py_top * width + px])
//                 } else {
//                     None
//                 };

//                 let bottom_pixel = if py_bottom < height && px < width {
//                     Some(pixels[py_bottom * width + px])
//                 } else {
//                     None
//                 };

//                 let cell = buf.cell_mut((x as u16, y as u16));
//                 if let Some(cell) = cell {
//                     match (top_pixel, bottom_pixel) {
//                         (Some(top), Some(bottom)) => {
//                             let top_color = Color::Rgb(top.r(), top.g(), top.b());
//                             let bottom_color = Color::Rgb(bottom.r(), bottom.g(), bottom.b());
//                             // Use half-block character: ▀ (upper half block)
//                             // Foreground color = top pixel, Background color = bottom pixel
//                             cell.set_char('▀');
//                             cell.set_fg(top_color);
//                             cell.set_bg(bottom_color);
//                         }
//                         (Some(top), None) => {
//                             let top_color = Color::Rgb(top.r(), top.g(), top.b());
//                             cell.set_char('▀');
//                             cell.set_fg(top_color);
//                         }
//                         (None, Some(bottom)) => {
//                             let bottom_color = Color::Rgb(bottom.r(), bottom.g(), bottom.b());
//                             cell.set_char('▄');
//                             cell.set_fg(bottom_color);
//                         }
//                         (None, None) => {}
//                     }
//                 }
//             }
//         }
//     }
// }
