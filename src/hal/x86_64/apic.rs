//! APIC (xAPIC): Local APIC + IOAPIC, sem dependência do PIC 8259.
//!
//! - Habilita xAPIC via MSR IA32_APIC_BASE.
//! - Mapeia MMIO (LAPIC/IOAPIC) pelo MM do kernel (NX, WRITABLE).
//! - Calibra o timer do LAPIC usando TSC e CPUID(0x16) e programa modo periódico.
//! - Roteia teclado (IRQ1) via IOAPIC para o vetor 0x21. Timer usa LAPIC no vetor 0x20.

use raw_cpuid::CpuId;
use core::arch::x86_64::_rdtsc;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::structures::paging::PageTableFlags;

const LAPIC_BASE_DEFAULT: u64 = 0xFEE0_0000;
const IOAPIC_BASE_DEFAULT: u64 = 0xFEC0_0000;

// LAPIC registers offsets
const LAPIC_ID: u32 = 0x20; // Local APIC ID Register
const LAPIC_EOI: u32 = 0xB0;
const LAPIC_SVR: u32 = 0xF0;
const LAPIC_LVT_TIMER: u32 = 0x320;
const LAPIC_TIMER_INITIAL: u32 = 0x380;
const LAPIC_TIMER_CURRENT: u32 = 0x390;
const LAPIC_TIMER_DIV: u32 = 0x3E0;

// IOAPIC registers
const IOAPIC_IOREGSEL: u32 = 0x00;
const IOAPIC_IOWIN: u32 = 0x10;

// Spurious interrupt vector
const SPURIOUS_VECTOR: u8 = 0xFF;

pub struct Lapic {
    base: *mut u32,
}

pub struct IoApic {
    base: *mut u32,
}

#[inline(always)]
unsafe fn mmio32(addr: u64) -> *mut u32 {
    addr as *mut u32
}

impl Lapic {
    pub unsafe fn new(base: u64) -> Self {
        Self { base: mmio32(base) }
    }

    unsafe fn write(&self, offset: u32, value: u32) {
        let reg = self.base.add((offset / 4) as usize);
        write_volatile(reg, value);
    }

    unsafe fn read(&self, offset: u32) -> u32 {
        let reg = self.base.add((offset / 4) as usize);
        read_volatile(reg)
    }

    pub unsafe fn eoi(&self) {
        self.write(LAPIC_EOI, 0);
    }

    pub unsafe fn enable(&self) {
        // bit 8 (APIC Software Enable) + spurious vector
        self.write(LAPIC_SVR, (SPURIOUS_VECTOR as u32) | (1 << 8));
    }

    pub unsafe fn apic_id(&self) -> u8 {
        ((self.read(LAPIC_ID) >> 24) & 0xFF) as u8
    }

    /// Programa o timer do LAPIC em modo periódico com vetor e carga inicial.
    pub unsafe fn timer_periodic(&self, vector: u8, initial: u32, divide_code: u32) {
        // Divisor: 0x0=2, 0x1=4, 0x2=8, 0x3=16, 0x8=32, 0x9=64, 0xA=128, 0xB=1
        self.write(LAPIC_TIMER_DIV, divide_code);
        let lvt = (vector as u32) | (1 << 17); // bit 17 = periodic
        self.write(LAPIC_LVT_TIMER, lvt);
        self.write(LAPIC_TIMER_INITIAL, initial);
    }

    /// One-shot para calibração: carrega contador sem gerar IRQ.
    pub unsafe fn timer_one_shot(&self, initial: u32, divide_code: u32) {
        self.write(LAPIC_TIMER_DIV, divide_code);
        // Máscara LVT para não gerar IRQ durante calibração
        self.write(LAPIC_LVT_TIMER, 1 << 16);
        self.write(LAPIC_TIMER_INITIAL, initial);
    }

    pub unsafe fn timer_current(&self) -> u32 { self.read(LAPIC_TIMER_CURRENT) }
}

impl IoApic {
    pub unsafe fn new(base: u64) -> Self {
        Self { base: mmio32(base) }
    }

    unsafe fn write_reg(&self, reg: u8, value: u32) {
        let sel = self.base.add((IOAPIC_IOREGSEL / 4) as usize);
        let win = self.base.add((IOAPIC_IOWIN / 4) as usize);
        write_volatile(sel, reg as u32);
        write_volatile(win, value);
    }

    unsafe fn read_reg(&self, reg: u8) -> u32 {
        let sel = self.base.add((IOAPIC_IOREGSEL / 4) as usize);
        let win = self.base.add((IOAPIC_IOWIN / 4) as usize);
        write_volatile(sel, reg as u32);
        read_volatile(win)
    }

    /// Programa uma entrada da tabela de redirecionamento do IOAPIC
    /// `irq` = IRQ ISA (0..15), `vector` = vetor IDT (ex.: 0x20), destino = APIC ID da CPU.
    pub unsafe fn route_isa_irq(&self, irq: u8, vector: u8, dest_apic_id: u8) {
        let idx = irq as u32;
        let low_reg = 0x10 + (2 * idx);
        let high_reg = low_reg + 1;
        // High: destination field nos bits 24..31
        self.write_reg(high_reg as u8, (dest_apic_id as u32) << 24);
        // Low: vector | delivery mode fixed | dest phys | active high | edge | unmask
        let low_val: u32 = (vector as u32) & 0xFF;
        self.write_reg(low_reg as u8, low_val);
    }
}

static LAPIC_BASE_VA: AtomicU64 = AtomicU64::new(0);
static IOAPIC_BASE_VA: AtomicU64 = AtomicU64::new(0);

/// Inicializa xAPIC (via MSR), mapeia MMIO via MM, calibra timer do LAPIC e programa IOAPIC.
pub unsafe fn init_apic() {
    enable_xapic_via_msr(LAPIC_BASE_DEFAULT);

    // Mapeia MMIOs com NX
    let lapic_va = crate::nt::mm::map_mmio(LAPIC_BASE_DEFAULT, 0x1000, PageTableFlags::NO_EXECUTE);
    let ioapic_va = crate::nt::mm::map_mmio(IOAPIC_BASE_DEFAULT, 0x20, PageTableFlags::NO_EXECUTE);
    LAPIC_BASE_VA.store(lapic_va, Ordering::SeqCst);
    IOAPIC_BASE_VA.store(ioapic_va, Ordering::SeqCst);

    let lapic = Lapic::new(lapic_va);
    let ioapic = IoApic::new(ioapic_va);
    lapic.enable();

    // Calibra timer do LAPIC (TSC/CPUID) e configura periódico no vetor 0x20
    let (ticks_per_ms, div_code) = calibrate_lapic_timer(&lapic);
    lapic.timer_periodic(0x20, ticks_per_ms, div_code);

    // Roteia teclado (IRQ1) -> vetor 0x21
    let apic_id = lapic.apic_id();
    ioapic.route_isa_irq(1, 0x21, apic_id);
}

pub unsafe fn apic_eoi_quick() {
    let base = LAPIC_BASE_VA.load(Ordering::SeqCst);
    if base != 0 {
        let reg = (base as *mut u32).add((LAPIC_EOI / 4) as usize);
        write_volatile(reg, 0);
    }
}

// --- Suporte: MSR e calibração ---
unsafe fn enable_xapic_via_msr(apic_phys_base: u64) {
    use x86_64::registers::model_specific::Msr;
    const IA32_APIC_BASE: u32 = 0x1B;
    let mut msr = Msr::new(IA32_APIC_BASE);
    let mut val = msr.read();
    // Base física (bits 12..35), APIC enable (bit 11), x2APIC off (bit 10)
    val &= !((0xFFFFF as u64) << 12);
    val |= (apic_phys_base & 0xFFFFF000) as u64;
    val |= 1 << 11;
    val &= !(1 << 10);
    msr.write(val);
}

/// Retorna (ticks_por_ms, divisor_escrito)
unsafe fn calibrate_lapic_timer(lapic: &Lapic) -> (u32, u32) {
    // Frequência base via CPUID 0x16 (MHz). Fallback para 3000 MHz em ambientes sem suporte
    let cpuid = CpuId::new();
    let cpu_mhz = cpuid
        .get_processor_frequency_info()
        .map(|fi| fi.processor_base_frequency() as u64)
        .unwrap_or(3000);

    // 10 ms em ciclos TSC (ciclos por µs = MHz)
    let target_us: u64 = 10_000;
    let delta_tsc: u64 = cpu_mhz * target_us;

    // Downcount do LAPIC em one-shot mascarado
    let div_code = 0x3; // divide por 16
    lapic.timer_one_shot(u32::MAX, div_code);
    let t0 = unsafe { _rdtsc() };
    while unsafe { _rdtsc() }.wrapping_sub(t0) < delta_tsc {}
    let cur = lapic.timer_current();
    let elapsed = (u32::MAX).wrapping_sub(cur);
    let ticks_per_us = (elapsed as u64) / target_us;
    let ticks_per_ms = (ticks_per_us * 1000).clamp(1, u32::MAX as u64) as u32;
    (ticks_per_ms, div_code)
}
