use bootloader_api::info::{FrameBuffer, FrameBufferInfo};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::fb_text;

lazy_static! {
    static ref CONSOLE: Mutex<Option<FbConsole>> = Mutex::new(None);
}

struct FbConsole {
    buffer_start: u64,
    info: FrameBufferInfo,
    x: usize,
    y: usize,
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
}

impl FbConsole {
    fn fb_mut(&mut self) -> FrameBuffer {
        unsafe { FrameBuffer::new(self.buffer_start, self.info) }
    }

    fn newline(&mut self) {
        let lh = 10usize; // line height
        self.x = 0;
        self.y = self.y.saturating_add(lh);
        if self.y + 8 >= self.info.height {
            // simple clear on overflow
            let mut fb = self.fb_mut();
            fb_text::clear(&mut fb, self.bg.0, self.bg.1, self.bg.2);
            self.x = 0;
            self.y = 0;
        }
    }

    fn write_char(&mut self, ch: char) {
        if ch == '\n' {
            self.newline();
            return;
        }
        if self.x + 8 >= self.info.width { self.newline(); }
        let mut fb = self.fb_mut();
        fb_text::draw_char(&mut fb, self.x, self.y, ch, self.fg, self.bg);
        self.x += 8;
    }

    fn write_str(&mut self, s: &str) {
        for ch in s.chars() { self.write_char(ch); }
    }
}

pub fn init_from_bootinfo(fb: &mut FrameBuffer) {
    let info = fb.info();
    let buffer_start = fb.buffer_mut().as_mut_ptr() as u64; // keep base ptr
    let mut guard = CONSOLE.lock();
    *guard = Some(FbConsole {
        buffer_start,
        info,
        x: 0,
        y: 0,
        fg: (255, 255, 255),
        bg: (0, 0, 0),
    });
    drop(guard);
}

pub fn clear(r: u8, g: u8, b: u8) {
    if let Some(console) = &mut *CONSOLE.lock() {
        console.bg = (r, g, b);
        let mut fb = console.fb_mut();
        fb_text::clear(&mut fb, r, g, b);
        console.x = 0;
        console.y = 0;
    }
}

pub fn log_str(s: &str) {
    if let Some(console) = &mut *CONSOLE.lock() {
        console.write_str(s);
        console.newline();
    }
}

pub fn log_char(c: char) {
    if let Some(console) = &mut *CONSOLE.lock() {
        console.write_char(c);
    }
}

