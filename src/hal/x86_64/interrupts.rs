//! HAL x86_64: IDT, IRQs e handlers de exceção.
//!
//! Regras importantes:
//! - ISR deve ser minimalista (ler porta/ack EOI). Nada de desenhar GUI/formatar string.
//! - Trabalho pesado vai para o laço principal (ex.: decodificar scancode e desenhar).
//! - Exceções fatais registram um resumo e entram em hlt_loop.

use crate::hal::x86_64::gdt;
use crate::serial_println;
use crate::drivers::video::fb_console;
use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::{
    instructions::{hlt, port::Port},
    registers::control::Cr2,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};
use pic8259::ChainedPics;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.general_protection_fault.set_handler_fn(gp_fault_handler);
        idt.stack_segment_fault.set_handler_fn(ss_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);
        for vec in 32u8..48u8 {
            if vec != InterruptIndex::Timer.as_u8() && vec != InterruptIndex::Keyboard.as_u8() {
                idt[vec].set_handler_fn(default_irq_handler);
            }
        }
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: BREAKPOINT");
    fb_console::log_str("EXCEPTION: BREAKPOINT");
}

extern "x86-interrupt" fn double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial_println!("EXCEPTION: DOUBLE FAULT");
    fb_console::log_str("EXCEPTION: DOUBLE FAULT");
    #[cfg(all(test, feature = "df_test"))]
    {
        crate::qemu::exit_qemu(crate::qemu::QemuExitCode::Success);
    }
    #[cfg(all(test, not(feature = "df_test")))]
    {
        crate::qemu::exit_qemu(crate::qemu::QemuExitCode::Failed);
    }
    hlt_loop()
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let addr = Cr2::read();
    serial_println!(
        "EXCEPTION: PAGE FAULT\r\nAccessed Address: {:?}\r\nError Code: {:?}\r\n{:#?}",
        addr, error_code, stack_frame
    );
    fb_console::log_str("EXCEPTION: PAGE FAULT (ver serial)");
    hlt_loop()
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: INVALID OPCODE\r\n{:#?}", stack_frame);
    fb_console::log_str("EXCEPTION: INVALID OPCODE");
    hlt_loop()
}

extern "x86-interrupt" fn gp_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    serial_println!("EXCEPTION: GENERAL PROTECTION FAULT ec={:#x}\r\n{:#?}", error_code, stack_frame);
    fb_console::log_str("EXCEPTION: GP FAULT");
    hlt_loop()
}

extern "x86-interrupt" fn ss_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    serial_println!("EXCEPTION: STACK SEGMENT FAULT ec={:#x}\r\n{:#?}", error_code, stack_frame);
    fb_console::log_str("EXCEPTION: SS FAULT");
    hlt_loop()
}

pub fn hlt_loop() -> ! {
    loop { hlt(); }
}

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 { self as u8 }
}

lazy_static! {
    pub static ref PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(
        Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore)
    );
}

pub unsafe fn init_pics() {
    PICS.lock().initialize();
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    #[cfg(feature = "apic")]
    unsafe {
        TICKS.fetch_add(1, Ordering::SeqCst);
        super::apic::apic_eoi_quick();
        return;
    }
    #[allow(unreachable_code)]
    unsafe { Port::<u8>::new(0x20).write(0x20); }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::<u8>::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    LAST_SCANCODE.store(scancode, Ordering::SeqCst);
    #[cfg(feature = "apic")]
    unsafe {
        super::apic::apic_eoi_quick();
        return;
    }
    #[allow(unreachable_code)]
    unsafe { Port::<u8>::new(0x20).write(0x20); }
}

extern "x86-interrupt" fn default_irq_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        Port::<u8>::new(0xA0).write(0x20);
        Port::<u8>::new(0x20).write(0x20);
    }
}

static LAST_SCANCODE: AtomicU8 = AtomicU8::new(0);
static TICKS: AtomicU64 = AtomicU64::new(0);

pub fn next_key_char() -> Option<char> {
    let sc = LAST_SCANCODE.swap(0, Ordering::SeqCst);
    if sc == 0 { return None; }
    let mut kbd = KEYBOARD.lock();
    if let Ok(Some(ev)) = kbd.add_byte(sc) {
        if let Some(key) = kbd.process_keyevent(ev) {
            if let DecodedKey::Unicode(ch) = key {
                return Some(ch);
            }
        }
    }
    None
}

pub fn tick_count() -> u64 {
    TICKS.load(Ordering::SeqCst)
}
