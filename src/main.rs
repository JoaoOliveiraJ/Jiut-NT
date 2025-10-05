#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use core::panic::PanicInfo;
use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use bootloader_api::config::Mapping;

mod hal;
mod drivers;
mod nt;
mod qemu;

// Solicita ao bootloader o mapeamento da memória física com offset dinâmico.
// Permite acesso inicial seguro a MMIO e criação do OffsetPageTable.
pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut cfg = BootloaderConfig::new_default();
    cfg.mappings.physical_memory = Some(Mapping::Dynamic);
    cfg
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // MM deve iniciar cedo para fornecer mapeamentos dedicados (heap, MMIO, stacks IST)
    serial_println!("init mm (NT)");
    unsafe { nt::mm::init(boot_info) };

    serial_println!("init gdt");
    hal::gdt::init();
    serial_println!("load idt");
    hal::interrupts::init_idt();

    // Inicializa e mascara completamente o PIC 8259 (sem uso com APIC)
    serial_println!("init pics (masked)");
    unsafe { hal::interrupts::init_pics(); }
    unsafe { hal::interrupts::PICS.lock().write_masks(0xFF, 0xFF); }

    // APIC completo (xAPIC + IOAPIC + LAPIC timer)
    #[cfg(feature = "apic")]
    unsafe {
        use crate::hal::x86_64::apic;
        apic::init_apic();
    }

    serial_println!("enable ints");
    x86_64::instructions::interrupts::enable();

    #[cfg(test)]
    test_main();

    if let Some(fb) = boot_info.framebuffer.as_mut() {
        crate::drivers::video::fb_console::init_from_bootinfo(fb);
        crate::drivers::video::fb_console::clear(0, 0, 0);
        crate::drivers::video::fb_console::log_str("Hello, World! (GUI)");
        crate::drivers::video::fb_console::log_str("init gdt");
        crate::drivers::video::fb_console::log_str("load idt");
        crate::drivers::video::fb_console::log_str("init pics (masked)");
        crate::drivers::video::fb_console::log_str("enable ints");
        crate::drivers::video::fb_console::log_str("enter idle loop");
    } else {
        println!("Hello, World!");
    }

    serial_println!("enter idle loop");
    loop {
        if let Some(ch) = hal::interrupts::next_key_char() {
            crate::drivers::video::fb_console::log_char(ch);
        }
        x86_64::instructions::hlt();
    }
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
