use std::io::{self, Write};
use std::process::Command;
use sysinfo::System;

fn clear_screen() {
    print!("{}[2J{}[1;1H", 27 as char, 27 as char);
    let _ = io::stdout().flush();
}

fn print_logo() {
    let logo = r#"
   ______                   ____  _____
  / ____/___  _________ _  / __ \/ ___/
 / /   / __ \/ ___/ __ `/ / / / /\__ \ 
/ /___/ /_/ / /  / /_/ / / /_/ /___/ / 
\____/\____/_/   \__,_/  \____//____/  
"#;

    // Print with a beautiful horizontal purple-to-cyan gradient
    let purple = "\x1b[38;2;168;85;247m";
    let cyan = "\x1b[38;2;6;182;212m";
    let reset = "\x1b[0m";

    println!();
    for (i, line) in logo.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        // Shift colors across lines
        if i % 2 == 0 {
            println!("{}{}{}", purple, line, reset);
        } else {
            println!("{}{}{}", cyan, line, reset);
        }
    }
    println!("  \x1b[1;37mWelcome to CoraOS (Debian GNU/Linux-based)\x1b[0m");
    println!("  \x1b[38;5;244m========================================\x1b[0m");
    println!();
}

fn get_uptime_string(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    
    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

fn display_dashboard(sys: &mut System) {
    sys.refresh_all();

    let os_name = sys.name().unwrap_or_else(|| "CoraOS".to_string());
    let kernel_ver = sys.kernel_version().unwrap_or_else(|| "Unknown".to_string());
    let host_name = sys.host_name().unwrap_or_else(|| "coraos".to_string());
    let uptime = System::uptime();
    
    let total_mem = sys.total_memory() / 1024 / 1024; // MB
    let used_mem = sys.used_memory() / 1024 / 1024; // MB
    let mem_percentage = if total_mem > 0 { (used_mem * 100) / total_mem } else { 0 };

    let cpus = sys.cpus();
    let cpu_brand = if !cpus.is_empty() {
        cpus[0].brand().trim().to_string()
    } else {
        "Unknown Processor".to_string()
    };

    println!("  \x1b[1;36mSYSTEM OVERVIEW\x1b[0m");
    println!("  ----------------------------------------");
    println!("  \x1b[1mOS Name:\x1b[0m      {}", os_name);
    println!("  \x1b[1mKernel:\x1b[0m       {}", kernel_ver);
    println!("  \x1b[1mHostname:\x1b[0m     {}", host_name);
    println!("  \x1b[1mUptime:\x1b[0m       {}", get_uptime_string(uptime));
    println!("  \x1b[1mCPU:\x1b[0m          {}", cpu_brand);
    println!("  \x1b[1mMemory:\x1b[0m       {} MB / {} MB ({}%)", used_mem, total_mem, mem_percentage);
    println!("  ----------------------------------------");
    println!();
}

fn run_command(cmd: &str, args: &[&str]) {
    clear_screen();
    println!("\x1b[1;33m[Running: {} {}]\x1b[0m\n", cmd, args.join(" "));
    let status = Command::new(cmd)
        .args(args)
        .status();

    match status {
        Ok(s) => {
            if !s.success() {
                println!("\n\x1b[1;31mCommand returned non-zero exit status.\x1b[0m");
            }
        }
        Err(e) => {
            println!("\n\x1b[1;31mFailed to execute command: {}\x1b[0m", e);
        }
    }
    
    print!("\nPress Enter to return to menu...");
    let _ = io::stdout().flush();
    let mut temp = String::new();
    let _ = io::stdin().read_line(&mut temp);
}

fn interactive_menu(sys: &mut System) {
    loop {
        clear_screen();
        print_logo();
        display_dashboard(sys);

        println!("  \x1b[1;35mMAIN MENU\x1b[0m");
        println!("  [1] Detailed Hardware Information");
        println!("  [2] Configure Network Interface (nmtui)");
        println!("  [3] Install/Update System Packages");
        println!("  [4] Launch Interactive Bash Shell");
        println!("  [5] System Power Options");
        println!("  [6] Log Out / Exit");
        println!();
        print!("  Enter your choice [1-6]: ");
        let _ = io::stdout().flush();

        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_err() {
            continue;
        }

        match choice.trim() {
            "1" => {
                clear_screen();
                println!("\x1b[1;36m=== Detailed Hardware Information ===\x1b[0m\n");
                sys.refresh_all();
                println!("  CPU Cores:     {}", sys.cpus().len());
                println!("  Total Memory:  {} MB", sys.total_memory() / 1024 / 1024);
                println!("  Used Memory:   {} MB", sys.used_memory() / 1024 / 1024);
                println!("  Total Swap:    {} MB", sys.total_swap() / 1024 / 1024);
                println!("  Used Swap:     {} MB", sys.used_swap() / 1024 / 1024);
                
                println!("\n  === CPU Brand Details ===");
                for (i, cpu) in sys.cpus().iter().enumerate() {
                    println!("    Core #{}: {} @ {:.0}MHz (Usage: {:.1}%)", 
                        i, cpu.brand(), cpu.frequency(), cpu.cpu_usage());
                }

                print!("\nPress Enter to return to menu...");
                let _ = io::stdout().flush();
                let mut temp = String::new();
                let _ = io::stdin().read_line(&mut temp);
            }
            "2" => {
                // nmtui is standard text-user-interface for NetworkManager in Debian
                run_command("nmtui", &[]);
            }
            "3" => {
                run_command("sudo", &["apt", "update"]);
                run_command("sudo", &["apt", "upgrade", "-y"]);
            }
            "4" => {
                clear_screen();
                println!("\x1b[1;32mDropping to bash shell... Type 'exit' to return to CoraOS welcome menu.\x1b[0m\n");
                let _ = Command::new("bash").status();
            }
            "5" => {
                loop {
                    clear_screen();
                    print_logo();
                    println!("  \x1b[1;31mPOWER OPTIONS\x1b[0m");
                    println!("  [1] Reboot System");
                    println!("  [2] Shut Down System");
                    println!("  [3] Go Back");
                    println!();
                    print!("  Select an option [1-3]: ");
                    let _ = io::stdout().flush();

                    let mut p_choice = String::new();
                    if io::stdin().read_line(&mut p_choice).is_err() {
                        continue;
                    }

                    match p_choice.trim() {
                        "1" => {
                            println!("\nRebooting now...");
                            let _ = Command::new("sudo").args(&["reboot"]).status();
                            return;
                        }
                        "2" => {
                            println!("\nShutting down now...");
                            let _ = Command::new("sudo").args(&["poweroff"]).status();
                            return;
                        }
                        "3" => break,
                        _ => {}
                    }
                }
            }
            "6" => {
                clear_screen();
                println!("Goodbye from CoraOS!");
                break;
            }
            _ => {}
        }
    }
}

fn main() {
    let mut sys = System::new_all();
    interactive_menu(&mut sys);
}
