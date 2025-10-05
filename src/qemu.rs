use x86_64::instructions::port::Port;

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(code: QemuExitCode) -> ! {
    // QEMU isa-debug-exit: exit status is (value << 1) | 1
    let mut port: Port<u32> = Port::new(0xF4);
    unsafe {
        port.write(code as u32);
    }
    loop {}
}

