//! Estrutura inspirada no NT: submódulos do Executive/Kernel.
pub mod ke; // Kernel Executive (scheduling, traps, IRQL)
pub mod mm; // Memory Manager
pub mod io; // I/O Manager
pub mod ps; // Process/Thread Manager
pub mod se; // Security
pub mod ob; // Object Manager

