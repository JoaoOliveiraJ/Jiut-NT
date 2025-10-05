Roadmap (NT-like Kernel)
=======================

Fase 1 — Sólido em 1 CPU (concluindo)
-------------------------------------
- APIC completo (xAPIC + IOAPIC) com timer LAPIC calibrado.
- PIC 8259 totalmente mascarado.
- MM com mapper, frame allocator, heap e guard pages.
- Scripts de build/execução com BIOS/QEMU.

Itens restantes para fechar a fase:
- [ ] Fallback de calibração do LAPIC timer (PIT/HPET) quando CPUID(0x16) indisponível
- [ ] IOAPIC: rotas adicionais e leitura de ID/versão para diagnóstico
- [ ] Limpeza do caminho antigo do PIC onde não for mais necessário

Fase 2 — Base de tempo, SMP e ACPI
----------------------------------
- [ ] ACPI: parse MADT (endereços LAPIC/IOAPIC, CPUs, overrides)
- [ ] SMP: iniciar APs, enviar IPIs, sincronização básica, per-CPU data
- [ ] Tempo: clock monotônico + scheduler tick configurável
- [ ] KE: IRQLs mínimos e deferral (DPC), sem bloquear em ISR

Fase 3 — MM avançado e drivers essenciais
-----------------------------------------
- [ ] MM físico: buddy allocator; MM virtual: slabs/arenas por tamanho
- [ ] Mapeador MMIO completo por API (adicionar atributos cache/UC conforme necessário)
- [ ] Drivers: teclado robusto, timer adicional (HPET ou TSC-Deadline), RTC básico

Fase 4 — IO/PCIe e interrupções modernas
----------------------------------------
- [ ] Enumeração PCI/PCIe
- [ ] MSI/MSI-X (quando suportado)
- [ ] Organização de stack de drivers (IO manager) e IRPs simplificados

Fase 5 — Boot UEFI e consolidação
---------------------------------
- [ ] Caminho UEFI paralelo ao BIOS
- [ ] Consolidação de logs (ring buffer) e tracing leve
- [ ] Testes básicos de regressão (smoke) e validações em QEMU

Notas
-----
- ISR minimalista: ler porta, sinalizar e EOI — todo trabalho pesado fora de ISR.
- Evitar GUI/serial em ISR; use buffers e processe no loop principal.
- Documentar decisões em `docs/STATUS.md`; manter este roadmap sincronizado.

