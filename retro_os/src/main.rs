//! RetroOS - A Lightweight Linux-like OS with Retro Mac OS UI
//! 
//! Features:
//! - Native Linux program execution (Priority #1)
//! - Retro Mac OS-style UI (Priority #2)
//! - WebAssembly support for browser execution (Priority #3)
//! - Future Windows and Android compatibility layers
//! - Designed for <128MB RAM at idle
//! - Supports x86, x86_64, ARM, and RISC-V architectures

mod kernel;
mod process;
mod fs;
mod ui;
mod compat;
mod wasm;

use anyhow::Result;
use log::{info, error};
use std::env;
use std::time::Instant;

use kernel::Kernel;
use process::ProcessManager;
use fs::FileSystem;
use ui::Desktop;
use compat::CompatManager;

/// RetroOS main application structure
pub struct RetroOS {
    kernel: Kernel,
    process_manager: ProcessManager,
    file_system: FileSystem,
    desktop: Desktop,
    compat_manager: CompatManager,
    wasm_runtime: wasm::WasmRuntime,
}

impl RetroOS {
    /// Create a new RetroOS instance
    pub fn new() -> Self {
        RetroOS {
            kernel: Kernel::new(),
            process_manager: ProcessManager::new(),
            file_system: FileSystem::new("/"),
            desktop: Desktop::new(),
            compat_manager: CompatManager::new(),
            wasm_runtime: wasm::WasmRuntime::new(),
        }
    }

    /// Boot the operating system
    pub async fn boot(&mut self) -> Result<()> {
        let boot_start = Instant::now();
        
        println!("╔════════════════════════════════════════╗");
        println!("║         RetroOS v{}           ║", env!("CARGO_PKG_VERSION"));
        println!("║  Lightweight OS with Retro Mac UI      ║");
        println!("╚════════════════════════════════════════╝");
        println!();

        // Initialize kernel
        info!("Initializing kernel...");
        self.kernel.init()?;
        println!("✓ Kernel initialized");

        // Check memory constraints
        if let Ok(mem_usage) = self.kernel.check_memory_usage() {
            println!("✓ Current memory usage: {}MB (target: <128MB)", mem_usage);
        }

        // Initialize filesystem
        info!("Initializing filesystem...");
        self.file_system = FileSystem::new("/");
        println!("✓ Filesystem ready");

        // Initialize compatibility manager
        info!("Initializing compatibility layer...");
        println!("✓ Native Linux execution enabled");
        println!("ℹ Windows/Android support: Coming soon");

        // Initialize WASM runtime if available
        #[cfg(not(feature = "wasm"))]
        {
            wasm::WasmRuntime::check_native_support();
        }
        
        #[cfg(feature = "wasm")]
        {
            self.wasm_runtime.init()?;
            println!("✓ WASM runtime initialized");
        }

        let boot_time = boot_start.elapsed();
        println!();
        println!("✓ System booted in {:.2?}", boot_time);
        println!();

        Ok(())
    }

    /// Run the shell interface
    pub async fn run_shell(&mut self) -> Result<()> {
        println!("RetroOS Shell v{}", env!("CARGO_PKG_VERSION"));
        println!("Type 'help' for commands, 'exit' to quit");
        println!();

        loop {
            print!("retro-os$ ");
            use std::io::{self, Write};
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            match input {
                "exit" | "quit" => {
                    info!("User requested exit");
                    break;
                }
                "help" => {
                    self.show_help();
                }
                "version" | "ver" => {
                    println!("RetroOS v{}", env!("CARGO_PKG_VERSION"));
                    println!("Kernel: {} v{}", kernel::KERNEL_NAME, kernel::KERNEL_VERSION);
                    println!("Uptime: {} seconds", self.kernel.uptime());
                }
                "mem" => {
                    match self.kernel.check_memory_usage() {
                        Ok(mem) => println!("Memory usage: {}MB", mem),
                        Err(e) => println!("Failed to get memory info: {}", e),
                    }
                }
                "ps" => {
                    let procs = self.process_manager.list_processes().await;
                    if procs.is_empty() {
                        println!("No active processes");
                    } else {
                        println!("Active processes: {}", procs.len());
                        for proc in procs {
                            println!("  PID {}: {} ({:?})", proc.pid, proc.name, proc.status);
                        }
                    }
                }
                "ui" => {
                    println!("Launching retro UI demo...");
                    if let Err(e) = ui::run_demo() {
                        error!("UI error: {}", e);
                    }
                }
                "ls" => {
                    match self.file_system.list_dir(".") {
                        Ok(entries) => {
                            for entry in entries {
                                println!("  {}", entry);
                            }
                        }
                        Err(e) => println!("Error: {}", e),
                    }
                }
                cmd if cmd.starts_with("run ") => {
                    let parts: Vec<&str> = cmd[4..].split_whitespace().collect();
                    if !parts.is_empty() {
                        let program = parts[0];
                        let args: Vec<&str> = parts[1..].to_vec();
                        
                        match self.process_manager.execute(program, &args).await {
                            Ok(pid) => {
                                println!("Started process {} (PID: {})", program, pid);
                                // Wait for completion
                                let exit_code = self.process_manager.wait_for_process(pid).await?;
                                println!("Process exited with code: {}", exit_code);
                            }
                            Err(e) => println!("Failed to execute: {}", e),
                        }
                    }
                }
                _ => {
                    // Try to execute as a command
                    let parts: Vec<&str> = input.split_whitespace().collect();
                    if !parts.is_empty() {
                        let program = parts[0];
                        let args: Vec<&str> = parts[1..].to_vec();
                        
                        match self.process_manager.execute(program, &args).await {
                            Ok(pid) => {
                                let _ = self.process_manager.wait_for_process(pid).await;
                            }
                            Err(e) => println!("Command not found or failed: {}", e),
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Show help message
    fn show_help(&self) {
        println!("Available commands:");
        println!("  help          - Show this help message");
        println!("  version       - Show system version");
        println!("  mem           - Show memory usage");
        println!("  ps            - List running processes");
        println!("  ls            - List current directory");
        println!("  run <cmd>     - Run a Linux program");
        println!("  ui            - Launch retro UI demo");
        println!("  exit          - Exit the shell");
    }

    /// Get system information
    pub fn get_info(&self) -> String {
        format!(
            "RetroOS v{}\n\
             Architecture: {}\n\
             Kernel: {} v{}\n\
             Memory Limit: {}MB\n\
             Processes: {}\n\
             Windows: {}",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::ARCH,
            kernel::KERNEL_NAME,
            kernel::KERNEL_VERSION,
            self.kernel.memory_limit_mb,
            0, // Would need async access
            self.desktop.window_count(),
        )
    }
}

impl Default for RetroOS {
    fn default() -> Self {
        Self::new()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let args: Vec<String> = env::args().collect();

    // Check for command-line arguments
    if args.len() > 1 {
        match args[1].as_str() {
            "--version" | "-v" => {
                println!("RetroOS v{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                println!("RetroOS v{}", env!("CARGO_PKG_VERSION"));
                println!("Usage: retro_os [OPTIONS] [COMMAND]");
                println!();
                println!("Options:");
                println!("  -v, --version    Show version");
                println!("  -h, --help       Show help");
                println!("  --ui             Launch UI demo");
                println!("  --shell          Launch shell (default)");
                println!();
                println!("Features:");
                println!("  • Native Linux program execution");
                println!("  • Retro Mac OS-style UI");
                println!("  • <128MB RAM target at idle");
                println!("  • Multi-architecture support (x86, ARM, RISC-V)");
                println!("  • Future: Windows & Android compatibility");
                println!("  • Future: WebAssembly browser support");
                return Ok(());
            }
            "--ui" => {
                let mut os = RetroOS::new();
                os.boot().await?;
                return ui::run_demo();
            }
            cmd => {
                // Execute command directly
                let mut os = RetroOS::new();
                os.boot().await?;
                
                let program = cmd;
                let args: Vec<&str> = args[2..].iter().map(|s| s.as_str()).collect();
                
                let pid = os.process_manager.execute(program, &args).await?;
                let exit_code = os.process_manager.wait_for_process(pid).await?;
                std::process::exit(exit_code);
            }
        }
    }

    // Default: launch interactive shell
    let mut os = RetroOS::new();
    os.boot().await?;
    os.run_shell().await?;

    Ok(())
}
