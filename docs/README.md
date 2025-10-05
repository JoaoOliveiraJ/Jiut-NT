Documentação do Kernel (NT-like)
===============================

Arquivos
--------
- `AGENTS.md`: Diretrizes para contribuição (ISR minimalista, GUI fora de ISR, etc.).
- `STATUS.md`: Changelog, pendências e próximos passos.
- `ARQUITETURA.md`: Visão geral da arquitetura NT-like.

Como rodar
----------
- Gerar imagem BIOS: `powershell -ExecutionPolicy Bypass -File scripts/build_bios.ps1`
- Rodar QEMU (serial stdio): `powershell -ExecutionPolicy Bypass -File scripts/run_qemu_bios.ps1`
- Logs do kernel aparecem na janela do QEMU (framebuffer) e na serial.

