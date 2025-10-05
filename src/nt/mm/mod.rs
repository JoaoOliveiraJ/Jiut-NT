//! Memory Manager (NT-like): mapper de páginas, frame allocator, MMIO, guard pages e heap.
//!
//! - OffsetPageTable ativo (a partir de CR3 + physical_memory_offset do bootloader).
//! - FrameAllocator baseado nas regiões `Usable` do mapa de memória do bootloader.
//! - API para mapear MMIO em VA alto dedicado.
//! - Heap do kernel (8 MiB) com allocator de lista ligada e coalescência simples.
//! - Alocação de stacks com guard page para IST (#DF) e outras pilhas internas.

use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use x86_64::structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB};
use x86_64::{PhysAddr, VirtAddr, registers::control::Cr3};
use bootloader_api::BootInfo;
use bootloader_api::info::MemoryRegionKind;

static MMAP: Mutex<Option<OffsetPageTable<'static>>> = Mutex::new(None);
static FRAME_ALLOC: Mutex<Option<BootInfoFrameAllocator>> = Mutex::new(None);

const MMIO_BASE: u64 = 0xFFFF_FF80_0000_0000; // região alta para MMIO
static NEXT_MMIO_VA: AtomicU64 = AtomicU64::new(MMIO_BASE);

const HEAP_START: u64 = 0xFFFF_FF00_0000_0000; // região alta dedicada ao heap do kernel
const HEAP_SIZE: usize = 8 * 1024 * 1024;      // 8 MiB

const STACKS_BASE: u64 = 0xFFFF_FF90_0000_0000; // região alta para stacks com guard page
static NEXT_STACK_VA: AtomicU64 = AtomicU64::new(STACKS_BASE);

pub unsafe fn init(boot_info: &BootInfo) {
    // 1) Mapper a partir do CR3 + physical_memory_offset
    let phys_off_opt: Option<u64> = boot_info.physical_memory_offset.into();
    let phys_off = phys_off_opt.unwrap_or(0);
    let mapper = init_offset_page_table(phys_off);
    *MMAP.lock() = Some(mapper);

    // 2) Frame allocator a partir do mapa de memória do bootloader
    let frame_alloc = BootInfoFrameAllocator::new(&boot_info.memory_regions);
    *FRAME_ALLOC.lock() = Some(frame_alloc);

    // 3) Heap do kernel
    init_heap().expect("heap init failed");
}

/// Retorna guardas para mapper e frame allocator.
fn mapper_and_alloc() -> (spin::MutexGuard<'static, Option<OffsetPageTable<'static>>>, spin::MutexGuard<'static, Option<BootInfoFrameAllocator>>) {
    (MMAP.lock(), FRAME_ALLOC.lock())
}

/// Mapeia uma região MMIO física para um endereço virtual alto dedicado.
/// Retorna o virtual address base (u64).
pub unsafe fn map_mmio(phys_start: u64, size: usize, flags: PageTableFlags) -> u64 {
    let pages = (size + 0xFFF) / 0x1000;
    let va_start = NEXT_MMIO_VA.fetch_add((pages as u64) * 0x1000, Ordering::SeqCst);
    let va = VirtAddr::new(va_start);
    let (mut mapper_opt, mut fa_opt) = mapper_and_alloc();
    let mapper = mapper_opt.as_mut().expect("mapper not initialized");
    let fa = fa_opt.as_mut().expect("frame allocator not initialized");

    for i in 0..pages {
        let page = Page::<Size4KiB>::containing_address(va + (i as u64) * 0x1000);
        let frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(phys_start + (i as u64) * 0x1000));
        let map_flags = flags | PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        mapper.map_to(page, frame, map_flags, fa).expect("map_to failed").flush();
    }
    va_start
}

/// Aloca uma stack com `pages` páginas e uma guard page inferior não mapeada.
/// Retorna (base_mapeada, topo).
pub unsafe fn alloc_stack_with_guard(pages: usize) -> (VirtAddr, VirtAddr) {
    let total = (pages as u64 + 1) * 0x1000;
    let base = NEXT_STACK_VA.fetch_add(total, Ordering::SeqCst);
    let map_base = VirtAddr::new(base + 0x1000); // primeira página é guard
    let top = VirtAddr::new(base + total);

    let (mut mapper_opt, mut fa_opt) = mapper_and_alloc();
    let mapper = mapper_opt.as_mut().expect("mapper not initialized");
    let fa = fa_opt.as_mut().expect("frame allocator not initialized");

    for i in 0..pages {
        let page = Page::<Size4KiB>::containing_address(map_base + (i as u64) * 0x1000);
        let frame = fa.allocate_frame().expect("out of frames for stack");
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;
        mapper.map_to(page, frame, flags, fa).expect("map stack page").flush();
    }
    (map_base, top)
}

unsafe fn init_offset_page_table(phys_mem_offset: u64) -> OffsetPageTable<'static> {
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = VirtAddr::new(phys.as_u64() + phys_mem_offset);
    let l4_table: &mut PageTable = &mut *virt.as_mut_ptr();
    OffsetPageTable::new(l4_table, VirtAddr::new(phys_mem_offset))
}

/// Frame allocator baseado no mapa de memória do bootloader (USABLE regions).
pub struct BootInfoFrameAllocator {
    memory_regions: *const bootloader_api::info::MemoryRegions,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub fn new(memory_regions: &bootloader_api::info::MemoryRegions) -> Self {
        Self { memory_regions: memory_regions as *const _, next: 0 }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> + '_ {
        // Safety: BootInfo e seu mapa de memória vivem por toda a execução do kernel.
        let mr = unsafe { &*self.memory_regions };
        mr.iter()
            .filter(|r| r.kind == MemoryRegionKind::Usable)
            .flat_map(|r| (r.start..r.end).step_by(0x1000))
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        if frame.is_some() { self.next += 1; }
        frame
    }
}

// Necessário porque armazenamos um ponteiro cru em um static Mutex
unsafe impl Send for BootInfoFrameAllocator {}

// --- Heap do kernel ---
mod heap_alloc {
    use super::*;
    use alloc::alloc::{GlobalAlloc, Layout};

    pub struct Locked<A> { inner: spin::Mutex<A> }
    impl<A> Locked<A> { pub const fn new(inner: A) -> Self { Self { inner: spin::Mutex::new(inner) } } }
    impl<A> core::ops::Deref for Locked<A> {
        type Target = spin::Mutex<A>;
        fn deref(&self) -> &Self::Target { &self.inner }
    }

    #[repr(C)]
    struct ListNode { size: usize, next: Option<&'static mut ListNode> }
    impl ListNode {
        const fn new(size: usize) -> Self { Self { size, next: None } }
        fn start_addr(&self) -> usize { self as *const _ as usize }
        fn end_addr(&self) -> usize { self.start_addr() + self.size }
    }

    pub struct LinkedListAllocator { head: ListNode }
    impl LinkedListAllocator {
        pub const fn new() -> Self { Self { head: ListNode { size: 0, next: None } } }
        pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
            self.add_free_region(heap_start, heap_size);
        }

        unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
            assert!(size >= core::mem::size_of::<ListNode>());
            let node_ptr = addr as *mut ListNode;
            node_ptr.write(ListNode::new(size));
            (*node_ptr).next = self.head.next.take();
            self.head.next = Some(&mut *node_ptr);
            self.coalesce();
        }

        fn align_up(addr: usize, align: usize) -> usize {
            (addr + align - 1) & !(align - 1)
        }

        unsafe fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
            let mut prev = &mut self.head as *mut ListNode;
            let mut current_opt = (*prev).next.as_mut().map(|r| &mut **r);
            while let Some(region) = current_opt {
                let alloc_start = Self::align_up(region.start_addr() + core::mem::size_of::<ListNode>(), align);
                let alloc_end = alloc_start.checked_add(size)?;
                if alloc_end <= region.end_addr() {
                    let next = region.next.take();
                    (*prev).next = next; // remove da lista
                    return Some((region, alloc_start));
                }
                prev = region as *mut _;
                current_opt = region.next.as_mut().map(|r| &mut **r);
            }
            None
        }

        unsafe fn coalesce(&mut self) {
            // Passo linear: mescla nós adjacentes por endereço quando possível
            let mut cur_ptr = self.head.next.as_mut().map(|r| &mut **r as *mut ListNode);
            while let Some(cur) = cur_ptr {
                let next_ptr = (*cur).next.as_mut().map(|r| &mut **r as *mut ListNode);
                if let Some(nxt) = next_ptr {
                    if (*cur).end_addr() == (*nxt).start_addr() {
                        (*cur).size += (*nxt).size;
                        (*cur).next = (*nxt).next.take();
                        continue; // tenta mesclar novamente com o próximo
                    }
                }
                cur_ptr = (*cur).next.as_mut().map(|r| &mut **r as *mut ListNode);
            }
        }
    }

    unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let mut alloc = self.lock();
            let size = layout.size().max(core::mem::size_of::<ListNode>());
            let align = layout.align().max(core::mem::align_of::<ListNode>());
            if let Some((region, alloc_start)) = alloc.find_region(size, align) {
                let alloc_end = alloc_start + size;
                // sobra antes
                let prefix_size = alloc_start - region.start_addr();
                if prefix_size >= core::mem::size_of::<ListNode>() {
                    alloc.add_free_region(region.start_addr(), prefix_size);
                }
                // sobra depois
                let suffix_size = region.end_addr() - alloc_end;
                if suffix_size >= core::mem::size_of::<ListNode>() {
                    alloc.add_free_region(alloc_end, suffix_size);
                }
                alloc.coalesce();
                alloc_start as *mut u8
            } else {
                core::ptr::null_mut()
            }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            let mut alloc = self.lock();
            alloc.add_free_region(ptr as usize, layout.size().max(core::mem::size_of::<ListNode>()));
            alloc.coalesce();
        }
    }

    #[global_allocator]
    pub static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());

    pub unsafe fn init_heap_range(start: usize, size: usize) {
        ALLOCATOR.lock().init(start, size);
    }
}

/// Mapeia páginas para o heap do kernel e inicializa o global allocator.
fn init_heap() -> Result<(), ()> {
    let heap_start = VirtAddr::new(HEAP_START);
    let heap_end = heap_start + (HEAP_SIZE as u64);
    let mut mapper_guard = MMAP.lock();
    let mut fa_guard = FRAME_ALLOC.lock();
    let mapper = mapper_guard.as_mut().ok_or(())?;
    let fa = fa_guard.as_mut().ok_or(())?;

    let mut page = Page::<Size4KiB>::containing_address(heap_start);
    let last_page = Page::<Size4KiB>::containing_address(heap_end - 1u64);
    while page <= last_page {
        let frame = fa.allocate_frame().ok_or(())?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;
        unsafe { mapper.map_to(page, frame, flags, fa).map_err(|_| ())?.flush() };
        page = Page::from_start_address(page.start_address() + 0x1000u64).unwrap();
    }

    unsafe { heap_alloc::init_heap_range(HEAP_START as usize, HEAP_SIZE) };
    Ok(())
}
