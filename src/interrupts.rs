use crate::{gdt, serial_print, serial_println};
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
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: BREAKPOINT");
}

extern "x86-interrupt" fn double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial_println!("EXCEPTION: DOUBLE FAULT");
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
    pub fn as_usize(self) -> usize { usize::from(self.as_u8()) }
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
    unsafe { PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8()); }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::<u8>::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    let mut kbd = KEYBOARD.lock();
    if let Ok(Some(event)) = kbd.add_byte(scancode) {
        if let Some(key) = kbd.process_keyevent(event) {
            match key {
                DecodedKey::Unicode(ch) => { serial_print!("{}", ch); }
                DecodedKey::RawKey(k) => { serial_print!("<{:?}>", k); }
            }
        }
    }

    unsafe { PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8()); }
}
