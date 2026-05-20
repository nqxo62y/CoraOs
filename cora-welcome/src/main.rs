use std::io::{self, Write};
use std::process::Command;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

const ESC: &str = "\x1b";

fn clear_screen() {
    print!("{ESC}[2J{ESC}[1;1H");
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

    let purple = "\x1b[38;2;168;85;247m";
    let cyan = "\x1b[38;2;6;182;212m";
    let reset = "\x1b[0m";

    println!();
    for (i, line) in logo.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if i % 2 == 0 {
            println!("{purple}{line}{reset}");
        } else {
            println!("{cyan}{line}{reset}");
        }
    }
    println!("  \x1b[1;37mCoraOS — Debian (server)\x1b[0m");
    println!("  \x1b[38;5;244m========================================\x1b[0m");
    println!();
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn refresh_dashboard(sys: &mut System) {
    sys.refresh_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );
}

fn print_dashboard(sys: &mut System) {
    refresh_dashboard(sys);

    let os_name = System::name().unwrap_or_else(|| "Linux".to_string());
    let kernel = System::kernel_version().unwrap_or_else(|| "?".to_string());
    let host = System::host_name().unwrap_or_else(|| "?".to_string());
    let uptime = System::uptime();

    let total_mib = sys.total_memory() / 1024 / 1024;
    let used_mib = sys.used_memory() / 1024 / 1024;
    let mem_pct = if total_mib > 0 {
        (used_mib * 100) / total_mib
    } else {
        0
    };

    let cpu_label = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "CPU".to_string());

    println!("  \x1b[1;36mSYSTEM\x1b[0m");
    println!("  ----------------------------------------");
    println!("  \x1b[1mOS:\x1b[0m       {os_name}");
    println!("  \x1b[1mKernel:\x1b[0m    {kernel}");
    println!("  \x1b[1mHost:\x1b[0m      {host}");
    println!("  \x1b[1mUptime:\x1b[0m    {}", format_uptime(uptime));
    println!("  \x1b[1mCPU:\x1b[0m       {cpu_label}");
    println!(
        "  \x1b[1mRAM:\x1b[0m       {used_mib} / {total_mib} MiB ({mem_pct}%)"
    );
    println!("  ----------------------------------------");
    println!();
}

fn pause() {
    print!("\nPress Enter to continue... ");
    let _ = io::stdout().flush();
    let mut buf = String::new();
    let _ = io::stdin().read_line(&mut buf);
}

fn run_command(cmd: &str, args: &[&str]) {
    clear_screen();
    println!(
        "\x1b[1;33m{cmd} {}\x1b[0m\n",
        args.iter().copied().collect::<Vec<_>>().join(" ")
    );

    match Command::new(cmd).args(args).status() {
        Ok(s) if !s.success() => println!("\n\x1b[1;31mExit code != 0\x1b[0m"),
        Ok(_) => {}
        Err(e) => println!("\n\x1b[1;31mFailed to start: {e}\x1b[0m"),
    }

    pause();
}

fn hardware_detail(sys: &mut System) {
    clear_screen();
    println!("\x1b[1;36mHardware Details\x1b[0m\n");

    refresh_dashboard(sys);

    println!("  Cores:       {}", sys.cpus().len());
    println!("  RAM total:   {} MiB", sys.total_memory() / 1024 / 1024);
    println!("  RAM used:    {} MiB", sys.used_memory() / 1024 / 1024);
    println!("  Swap total:  {} MiB", sys.total_swap() / 1024 / 1024);
    println!("  Swap used:   {} MiB", sys.used_swap() / 1024 / 1024);
    println!();

    for (i, cpu) in sys.cpus().iter().enumerate() {
        println!(
            "  #{i}: {} @ {:.0} MHz, {:.1}%",
            cpu.brand().trim(),
            cpu.frequency(),
            cpu.cpu_usage()
        );
    }

    pause();
}

fn power_menu() -> bool {
    loop {
        clear_screen();
        print_logo();
        println!("  \x1b[1;31mPOWER\x1b[0m");
        println!("  [1] Reboot");
        println!("  [2] Shutdown");
        println!("  [3] Back");
        print!("\n  Choice [1-3]: ");
        let _ = io::stdout().flush();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            continue;
        }

        match line.trim() {
            "1" => {
                let _ = Command::new("sudo").args(["reboot"]).status();
                return true;
            }
            "2" => {
                let _ = Command::new("sudo").args(["poweroff"]).status();
                return true;
            }
            "3" => return false,
            _ => {}
        }
    }
}

fn main_menu(sys: &mut System) {
    loop {
        clear_screen();
        print_logo();
        print_dashboard(sys);

        println!("  \x1b[1;35mMENU\x1b[0m");
        println!("  [1] Hardware (Detail)");
        println!("  [2] Network (ip -br a)");
        println!("  [3] Disks (df -hT)");
        println!("  [4] systemd (failed units)");
        println!("  [5] Journal (last 80 lines)");
        println!("  [6] apt update && apt full-upgrade -y");
        println!("  [7] Interactive shell (bash)");
        println!("  [8] Network TUI (nmtui)");
        println!("  [9] Power");
        println!("  [0] Exit");
        print!("\n  Choice [0-9]: ");
        let _ = io::stdout().flush();

        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_err() {
            continue;
        }

        match choice.trim() {
            "1" => hardware_detail(sys),
            "2" => run_command("ip", &["-br", "a"]),
            "3" => run_command("df", &["-hT"]),
            "4" => run_command("systemctl", &["--no-pager", "--failed"]),
            "5" => run_command("journalctl", &["-n", "80", "--no-pager"]),
            "6" => {
                run_command("sudo", &["apt", "update"]);
                run_command("sudo", &["apt", "full-upgrade", "-y"]);
            }
            "7" => {
                clear_screen();
                println!("\x1b[1;32mbash — type 'exit' to return\x1b[0m\n");
                let _ = Command::new("bash").status();
            }
            "8" => run_command("nmtui", &[]),
            "9" => {
                if power_menu() {
                    break;
                }
            }
            "0" => {
                clear_screen();
                break;
            }
            _ => {}
        }
    }
}

fn main() {
    let mut sys = System::new();
    main_menu(&mut sys);
}
