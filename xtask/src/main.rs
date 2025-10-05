use std::{path::PathBuf, process::Command};

use anyhow::{bail, Context, Result};
use bootloader::DiskImageBuilder;
use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(author, version, about = "Helper tasks for building/running the OS", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the kernel and generate a UEFI disk image
    Build {
        /// Release build
        #[arg(long)]
        release: bool,
        /// Output image path
        #[arg(long)]
        out: Option<PathBuf>,
        /// Build BIOS (MBR) image instead of UEFI
        #[arg(long)]
        bios: bool,
    },
    /// Cargo runner for `target_os = none` tests/binaries
    Runner(RunnerArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build { release, out, bios } => build_image(release, out, bios),
        Commands::Runner(args) => runner(args),
    }
}

fn project_root() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let mut p = exe.parent().context("no exe parent")?;
    // xtask/target/debug/xtask -> xtask/target/debug -> xtask/target -> xtask -> blog_os
    for _ in 0..3 {
        p = p.parent().context("invalid path structure")?;
    }
    Ok(p.to_path_buf())
}

fn build_kernel(release: bool) -> Result<PathBuf> {
    let root = project_root()?;
    let mut cmd = Command::new(cargo());
    cmd.current_dir(&root);
    cmd.env("RUSTFLAGS", "-Z unstable-options -C panic=immediate-abort");
    cmd.arg("build");
    // Build kernel package explicitly for our custom target using build-std
    cmd.args([
        "-Z",
        "build-std=core,compiler_builtins",
        "-Z",
        "build-std-features=compiler-builtins-mem",
        "--target",
        "x86_64-blog_os.json",
        "-p",
        "blog_os",
    ]);
    if release {
        cmd.arg("--release");
    }
    // Uses .cargo/config.toml for target and build-std
    let status = cmd.status().context("failed to spawn cargo build")?;
    if !status.success() {
        bail!("cargo build failed");
    }
    let target_dir = root.join("target").join("x86_64-blog_os");
    let profile = if release { "release" } else { "debug" };
    let kernel_path = target_dir.join(profile).join("blog_os");
    if !kernel_path.exists() {
        bail!("kernel binary not found at {}", kernel_path.display());
    }
    Ok(kernel_path)
}

fn build_image(release: bool, out: Option<PathBuf>, bios: bool) -> Result<()> {
    let kernel = build_kernel(release)?;
    let root = project_root()?;
    let (label, out) = if bios {
        ("BIOS", out.unwrap_or_else(|| root.join("target").join("bios.img")))
    } else {
        ("UEFI", out.unwrap_or_else(|| root.join("target").join("uefi.img")))
    };

    println!("building {} image -> {}", label, out.display());
    let builder = DiskImageBuilder::new(kernel);
    if bios {
        builder
            .create_bios_image(&out)
            .context("failed to create BIOS disk image")?;
    } else {
        builder
            .create_uefi_image(&out)
            .context("failed to create UEFI disk image")?;
    }
    println!("OK: {}", out.display());
    Ok(())
}

#[derive(Args, Debug)]
struct RunnerArgs {
    /// Use BIOS (MBR) image instead of UEFI
    #[arg(long)]
    bios: bool,
    /// Show QEMU window (GUI)
    #[arg(long)]
    gui: bool,
    /// The test kernel path passed by Cargo (first arg)
    #[arg()]
    kernel: PathBuf,
    /// Additional args (ignored, forwarded by cargo)
    #[arg(allow_hyphen_values = true)]
    rest: Vec<String>,
}

fn runner(args: RunnerArgs) -> Result<()> {
    let kernel = args.kernel.canonicalize().context("resolve kernel path")?;
    let out_img = std::env::temp_dir().join(if args.bios { "kernel-bios.img" } else { "kernel-uefi.img" });
    let mut builder = DiskImageBuilder::new(kernel);
    if args.bios {
        builder.create_bios_image(&out_img).context("create BIOS image")?;
    } else {
        builder.create_uefi_image(&out_img).context("create UEFI image")?;
    }
    let qemu = qemu_path()?;
    let mut cmd = std::process::Command::new(qemu);
    cmd.args([
        "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04",
        "-serial", "stdio",
        "-m", "256M",
    ]);
    if !args.gui {
        cmd.args(["-display", "none"]);
    }
    if args.bios {
        cmd.args(["-drive", &format!("format=raw,file={}", out_img.display())]);
    } else {
        let ovmf = ovmf_path()?;
        cmd.args(["-drive", &format!("if=pflash,format=raw,readonly=on,file={}", ovmf.display())]);
        cmd.args(["-drive", &format!("format=raw,file={}", out_img.display())]);
    }

    let status = cmd.status().context("run qemu")?;
    let code = status.code().unwrap_or(1);
    // isa-debug-exit returns (value << 1) | 1
    let decoded = (code >> 1) & 0xFF;
    if decoded == (crate::qemu_codes::SUCCESS as i32) {
        Ok(())
    } else {
        bail!("qemu exited with code {} (decoded {})", code, decoded)
    }
}

mod qemu_codes {
    pub const SUCCESS: u8 = 0x10;
    pub const FAILED: u8 = 0x11;
}

fn qemu_path() -> Result<String> {
    let candidates = [
        "C\\Program Files\\qemu\\qemu-system-x86_64.exe",
        "C:\\Program Files\\qemu\\qemu-system-x86_64.exe",
    ];
    for c in candidates {
        if std::path::Path::new(c).exists() { return Ok(c.into()); }
    }
    Ok("qemu-system-x86_64".into())
}

fn ovmf_path() -> Result<std::path::PathBuf> {
    let p = std::path::Path::new("C:/Program Files/qemu/share/edk2-x86_64-code.fd");
    if p.exists() { Ok(p.to_path_buf()) } else { bail!("OVMF not found: {}", p.display()) }
}

fn cargo() -> String {
    // Prefer explicit cargo path if available (Windows rustup path)
    if let Ok(home) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(home).join(".cargo").join("bin").join("cargo.exe");
        if p.exists() {
            return p.to_string_lossy().into();
        }
    }
    "cargo".into()
}
