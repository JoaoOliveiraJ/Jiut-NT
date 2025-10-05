#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use bootloader_api::{entry_point, BootInfo};
mod vga_buffer;
mod fb_text;
mod fb_console;
mod serial;
mod qemu;
mod gdt;
mod interrupts;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    serial_println!("init gdt");
    gdt::init();
    serial_println!("load idt");
    interrupts::init_idt();
    serial_println!("init pics");
    unsafe { interrupts::init_pics(); }
    // Unmask only keyboard (IRQ1); mask timer (IRQ0) to isolate issue
    unsafe { interrupts::PICS.lock().write_masks(0xFD, 0xFF); }
    serial_println!("enable ints");
    x86_64::instructions::interrupts::enable();
    
    #[cfg(test)]
    test_main();

    if let Some(fb) = boot_info.framebuffer.as_mut() {
        fb_console::init_from_bootinfo(fb);
        fb_console::clear(0, 0, 0);
        fb_console::log_str("Hello, World! (GUI)");
        fb_console::log_str("init gdt");
        fb_console::log_str("load idt");
        fb_console::log_str("init pics");
        fb_console::log_str("enable ints");
        fb_console::log_str("enter idle loop");
    } else {
        println!("Hello, World!");
    }
    serial_println!("enter idle loop");
    loop { x86_64::instructions::hlt(); }
}

#[panic_handler]
#[cfg(not(test))]
fn panic(info: &PanicInfo) -> ! {
    println!("PANIC: {:?}", info);
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed] kernel panic (test): {:?}", info);
    qemu::exit_qemu(qemu::QemuExitCode::Failed);
    loop {}
}

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    serial_print!("running {} tests...\r\n", tests.len());
    for test in tests {
        test();
        serial_println!("[ok]");
    }
    qemu::exit_qemu(qemu::QemuExitCode::Success);
}

#[cfg(test)]
#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}

#[cfg(test)]
#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}

#[cfg(all(test, feature = "df_test"))]
fn stack_overflow() {
    // prevent tail recursion elimination
    stack_overflow();
    core::hint::black_box(0u64);
}

#[cfg(all(test, feature = "df_test"))]
#[test_case]
fn test_stack_overflow_triggers_double_fault() {
    stack_overflow();
}
