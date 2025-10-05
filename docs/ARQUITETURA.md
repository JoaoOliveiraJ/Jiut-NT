Arquitetura (NT-like)
=====================

Camadas
-------
- HAL (`src/hal/x86_64`): dependente de arquitetura. GDT/IDT/TSS, IRQs, serial, APIC.
- Drivers (`src/drivers`): dispositivos e subsistemas (vídeo framebuffer, VGA, etc.).
- NT (`src/nt`): esqueleto de subsistemas do Windows NT: KE, MM, IO, PS, SE, OB.
- Kernel (`src/main.rs`): entrada, bootstrap, integração HAL/Drivers/NT.

HAL x86_64
----------
- GDT/TSS: segmento de código do kernel; IST para #DF usando stack estática 16B aligned (guard pages disponíveis para outras stacks internas).
- IDT: handlers de exceções (BP/PF/DF/UD/GP/SS) e de IRQs (timer/teclado).
- PIC 8259: inicializado e totalmente mascarado quando APIC está habilitado.
- Serial: UART 16550 (COM1 0x3F8) para logs de depuração.
- APIC: xAPIC via MSR IA32_APIC_BASE; IOAPIC roteando IRQ1 (teclado) para 0x21; timer via LAPIC com calibração TSC/CPUID; EOI via LAPIC.

Drivers de Vídeo
----------------
- Framebuffer (`fb_console`): logs diretamente na GUI (8x8 fonte). Evite uso em ISR.
- VGA texto (`vga_text`): legado (0xb8000); útil como fallback.

NT (Stubs)
----------
- KE (Kernel Executive): IRQL, traps, DPC/APC, agendador. (a definir)
- MM (Memory Manager): mapper/frames, MMIO, heap do kernel, guard pages.
- IO (I/O Manager): pilha de drivers, IRPs. (a definir)
- PS (Process/Thread Manager): processos/threads user-mode. (a definir)
- SE (Security): tokens, ACLs. (a definir)
- OB (Object Manager): namespace de objetos/handles. (a definir)

Diretrizes de Implementação
---------------------------
- ISR minimalista: ler porta e EOI; nada de formatação/desenho em interrupção.
- Logs GUI apenas no caminho de thread (event loop do kernel).
- Documente novas decisões em `docs/STATUS.md` e `docs/AGENTS.md`.

