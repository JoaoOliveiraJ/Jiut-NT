Este repositório contém um kernel em estilo NT (Windows NT-like). Regras para agentes/IA:

1) Organização do código
- `src/hal/x86_64`: código dependente de arquitetura (GDT/IDT/TSS, IRQs, serial, APIC, etc.).
- `src/drivers`: drivers genéricos (ex.: vídeo/framebuffer, VGA text).
- `src/nt`: esqueleto dos subsistemas NT (KE, MM, IO, PS, SE, OB).
- `src/main.rs`: ponto de entrada do kernel.

2) Interrupções e exceções
- ISR deve ser minimalista (ler porta, enviar EOI). NÃO desenhar na GUI, NÃO formatar strings no ISR.
- Mova trabalho pesado para o laço principal do kernel.
- Exceções fatais: log serial + linha na GUI e `hlt_loop()`.

3) Logs e GUI
- Use `drivers::video::fb_console` para mensagens visíveis em GUI.
- Evite usar `fb_console` dentro de IRQs; prefira filas/átomos e desenhe no loop principal.

4) Estabilidade
- APIC ativo por padrão: xAPIC (MSR), IOAPIC, timer via LAPIC; PIC 8259 totalmente mascarado.

5) Documentação
- Atualize `docs/STATUS.md` ao concluir marcos: o que foi feito, pendências, próximos passos.
- Comentários de topo por arquivo e função; evite comentários redundantes linha-a-linha.

6) Execução local
- Use `scripts/build_bios.ps1` e `scripts/run_qemu_bios.ps1` para gerar imagem e rodar o kernel no QEMU (serial stdio + GUI).

