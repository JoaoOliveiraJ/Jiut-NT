use lazy_static::lazy_static;
use x86_64::{
    instructions::tables::load_tss,
    registers::segmentation::{CS, Segment as _, SegmentSelector},
    structures::{gdt::Descriptor, gdt::GlobalDescriptorTable, tss::TaskStateSegment},
};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        // Stack de DF: buffer estático 16-byte aligned (guard pages podem ser usadas em outras stacks)
        let mut tss = TaskStateSegment::new();
        let stack_start = x86_64::VirtAddr::from_ptr(unsafe { core::ptr::addr_of!(DOUBLE_FAULT_STACK) });
        let stack_end = stack_start + (core::mem::size_of::<DfStack>() as u64);
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = stack_end;
        tss
    };

    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        // index 1: kernel code
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        // index 2: kernel data (necessário para SS válido em ring 0)
        let _data_selector = gdt.append(Descriptor::kernel_data_segment());
        // próximo: TSS (ocupa 2 entries)
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { code_selector, tss_selector })
    };
}

pub fn init() {
    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

#[repr(align(16))]
struct DfStack([u8; 4096 * 5]);

static mut DOUBLE_FAULT_STACK: DfStack = DfStack([0; 4096 * 5]);

