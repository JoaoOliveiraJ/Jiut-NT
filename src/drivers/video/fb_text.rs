use bootloader_api::info::{FrameBuffer, PixelFormat};

#[inline]
fn put_pixel(fb: &mut FrameBuffer, x: usize, y: usize, r: u8, g: u8, b: u8) {
    let info = fb.info();
    if x >= info.width || y >= info.height { return; }
    let bytes_per_pixel = info.bytes_per_pixel;
    let stride = info.stride;
    let idx = (y * stride + x) * bytes_per_pixel;
    let buf = fb.buffer_mut();
    match info.pixel_format {
        PixelFormat::Bgr => {
            if bytes_per_pixel >= 3 {
                buf[idx] = b;
                buf[idx + 1] = g;
                buf[idx + 2] = r;
            }
            if bytes_per_pixel == 4 { buf[idx + 3] = 0; }
        }
        PixelFormat::Rgb => {
            if bytes_per_pixel >= 3 {
                buf[idx] = r;
                buf[idx + 1] = g;
                buf[idx + 2] = b;
            }
            if bytes_per_pixel == 4 { buf[idx + 3] = 0; }
        }
        PixelFormat::U8 => {
            // grayscale approximation
            let y = ((r as u16 + g as u16 + b as u16) / 3) as u8;
            buf[idx] = y;
        }
        PixelFormat::Unknown { .. } => {
            // best-effort: write grayscale to first byte(s)
            let y = ((r as u16 + g as u16 + b as u16) / 3) as u8;
            for i in 0..bytes_per_pixel { buf[idx + i] = y; }
        }
        _ => { /* future formats */ }
    }
}

pub fn clear(fb: &mut FrameBuffer, r: u8, g: u8, b: u8) {
    let info = fb.info();
    for y in 0..info.height {
        for x in 0..info.width {
            put_pixel(fb, x, y, r, g, b);
        }
    }
}

pub fn draw_char(fb: &mut FrameBuffer, x: usize, y: usize, ch: char, fg: (u8,u8,u8), bg: (u8,u8,u8)) {
    let idx = if (ch as u32) < 256 { ch as usize } else { b'?' as usize };
    let glyph: [u8; 8] = font8x8::legacy::BASIC_LEGACY[idx];
    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..8 {
            let bit = (bits >> col) & 1;
            let (r,g,b) = if bit != 0 { fg } else { bg };
            put_pixel(fb, x + col, y + row, r, g, b);
        }
    }
}

pub fn draw_str(fb: &mut FrameBuffer, mut x: usize, mut y: usize, s: &str, fg: (u8,u8,u8), bg: (u8,u8,u8)) {
    let info = fb.info();
    let cw = 8usize; let ch = 8usize;
    for ch_ in s.chars() {
        match ch_ {
            '\n' => { y += ch; x = 0; },
            _ => {
                if x + cw >= info.width { y += ch; x = 0; }
                if y + ch >= info.height { break; }
                draw_char(fb, x, y, ch_, fg, bg);
                x += cw;
            }
        }
    }
}
