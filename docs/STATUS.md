Status do Kernel (NT-like)
=========================

Visão Geral
-----------
- Boot BIOS → modo longo (x86_64), kernel com GDT/IDT/TSS (IST para Double Fault).
- HAL x86_64 com APIC completo: xAPIC via MSR, IOAPIC roteando IRQs; timer via LAPIC.
- PIC 8259 inicializado e totalmente mascarado (sem dependência).
- Logs na GUI via framebuffer (fb_console); teclado aparece na tela.
- Estrutura NT (stubs): `nt/{ke, mm, io, ps, se, ob}`.

O que foi feito (changelog)
---------------------------
- APIC (completo):
  - Habilitação xAPIC via MSR IA32_APIC_BASE (bit 11) e base física padrão.
  - Mapeamento MMIO do LAPIC/IOAPIC com `nt::mm::map_mmio` (PRESENT/WRITABLE/NX), sem depender de offset do bootloader.
  - Calibração do timer do LAPIC utilizando TSC + CPUID(0x16); configuração periódica no vetor 0x20.
  - IOAPIC programado para rotear IRQ1 (teclado) para o vetor 0x21; ISRs fazem EOI via LAPIC.
  - PIC 8259 permanece totalmente mascarado durante operação (migração concluída).
- MM (completo):
  - OffsetPageTable a partir do CR3 + `physical_memory_offset` do bootloader.
  - FrameAllocator percorrendo regiões `Usable` do mapa de memória do bootloader.
  - Heap do kernel (8 MiB) com alocador de lista ligada e coalescência simples.
  - API `map_mmio` para VA alto dedicado; API `alloc_stack_with_guard` disponível para stacks com guard page.
  - IST de #DF usa stack estática alinhada a 16B (guard pages reservadas para outras stacks controladas pelo kernel).
- Ordem de inicialização revisada:
  - MM → GDT → IDT → PIC (mascarado) → APIC (xAPIC+IOAPIC) → habilitar interrupções.
- Scripts de build/execução:
  - `scripts/build_bios.ps1`: gera `target/bios.img` do kernel.
  - `scripts/run_qemu_bios.ps1`: (re)gera a imagem e roda QEMU com serial stdio.

Como rodar
----------
- Gerar imagem BIOS: `powershell -ExecutionPolicy Bypass -File scripts/build_bios.ps1`
- Rodar QEMU (serial stdio): `powershell -ExecutionPolicy Bypass -File scripts/run_qemu_bios.ps1`

Pendências atuais
-----------------
- Refinar calibração do LAPIC timer quando CPUID(0x16) não estiver disponível (fallback PIT/HPET).
- Expandir o roteamento do IOAPIC para outras IRQs (mouse, storage, etc.).
- MM avançado: buddy allocator físico, slabs/arenas por tamanho; mapeamento p2m dinâmico.
- SMP: iniciar APs, enviar IPIs, balanceamento simples.

Diretrizes rápidas
------------------
- ISR minimalista (ler porta, EOI). Não desenhar/formatar na ISR; use o laço principal.
- Atualize este arquivo ao concluir marcos e documente decisões.

Roadmap (organizado)
--------------------
- Curto prazo (1–2 dias)
  - LAPIC: fallback de calibração (PIT/HPET) quando CPUID(0x16) indisponível.
  - IOAPIC: rotas adicionais (ex.: IRQ12/mouse) e verificação de ID/versão.
  - Limpeza: remover caminho antigo do PIC 8259 onde não for mais necessário.
- Médio prazo (1–2 semanas)
  - ACPI: parse MADT para descobrir LAPIC/IOAPIC e CPUs; preparar SMP real.
  - SMP: iniciar APs, configurar per-CPU structures, enviar IPIs, EOI per-CPU.
  - MM: buddy allocator físico + slabs; API de mapeamento dinâmico p2m/mmio.
  - Tempo: clock monotônico, scheduler tick configurável, base para timers do kernel.
  - Consolidação de logs: ring buffer e serial/GUI integrados (sem IO em ISR).
- Longo prazo
  - Scheduler: run queues por CPU, prioridades simples, DPC/APC (KE).
  - IO: enumeração PCI/PCIe, drivers base (timer HPET/APIC-TSC-Deadline, teclado, rtc).
  - UEFI boot: fluxo paralelo ao BIOS e manutenção dos dois caminhos.
  - MSI/MSI-X: habilitar quando usar PCIe e dispositivos modernos.
