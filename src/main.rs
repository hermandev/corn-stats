use std::{collections::HashMap, fs, process::Command, time::Duration};

use appindicator3::{Indicator, traits::AppIndicatorExt};
use clap::{Parser, Subcommand};
use gtk::prelude::*;
use sysinfo::{Networks, System};

const SERVICE_PATH: &str = "/etc/systemd/system/corn_stats.service";
const GLOBAL_BIN: &str = "/usr/local/bin/corn_stats";

fn main() {
    if initialize_cli() {
        return;
    }

    gtk::init().unwrap();

    let menu = gtk::Menu::new();
    let quit = gtk::MenuItem::with_label("Quit");

    quit.connect_activate(|_| gtk::main_quit());

    menu.append(&quit);
    menu.show_all();

    let indicator = Indicator::new(
        "id-codecorn-corn_stats",
        "...",
        appindicator3::IndicatorCategory::ApplicationStatus,
    );

    indicator.set_status(appindicator3::IndicatorStatus::Active);
    indicator.set_menu(Some(&menu));

    let mut sys = System::new_all();
    let mut networks = Networks::new_with_refreshed_list();
    let mut prev_rx_tx = HashMap::new();

    glib::timeout_add_local(Duration::from_millis(800), move || {
        networks.refresh(true);
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        let mut total_rx = 0u64;
        let mut total_tx = 0u64;

        for (_name, net) in &networks {
            total_rx += net.received();
            total_tx += net.transmitted();
        }

        let rx_speed = total_rx.saturating_sub(*prev_rx_tx.get("rx").unwrap_or(&0));
        let tx_speed = total_tx.saturating_sub(*prev_rx_tx.get("tx").unwrap_or(&0));

        prev_rx_tx.insert("rx", total_rx);
        prev_rx_tx.insert("tx", total_tx);

        let cpu = sys.global_cpu_usage();
        let used = sys.used_memory() as f64;
        let total = sys.total_memory() as f64;

        let mem_percent = (used / total) * 100.0;

        indicator.set_label(
            &format!(
                "⚡{:.0}% · 🧠{:.0}% · ↓{:.1}KB/s ↑{:.1}KB/s",
                cpu,
                mem_percent,
                rx_speed as f64 / 1024.0,
                tx_speed as f64 / 1024.0
            ),
            "",
        );
        glib::ControlFlow::Continue
    });

    gtk::main();
}

#[derive(Parser)]
#[command(name = "corn_stats")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Install {
        #[arg(long)]
        global: bool,
    },
    Uninstall,
    Start,
    Stop,
}

fn initialize_cli() -> bool {
    if std::env::args().len() <= 1 {
        return false;
    }
    let cli = Cli::parse();

    match cli.command {
        Commands::Install { global } => install(global),
        Commands::Uninstall => uninstall(),
        Commands::Start => start(),
        Commands::Stop => stop(),
    }

    true
}

fn ensure_root() {
    if !nix::unistd::geteuid().is_root() {
        eprintln!("This command must be run as root (use sudo).");
        std::process::exit(1);
    }
}

fn install(global: bool) {
    ensure_root();
    let current_bin = std::env::current_exe().unwrap();

    if global {
        let current_path = current_bin.to_string_lossy();
        if current_path != GLOBAL_BIN {
            println!("Installing globally to {}", GLOBAL_BIN);
            fs::copy(&current_bin, GLOBAL_BIN).expect("Failed to copy binary to /usr/local/bin");
        } else {
            println!("Binary already running from {}", GLOBAL_BIN);
        }
    }

    let exec_path = if global {
        GLOBAL_BIN.to_string()
    } else {
        current_bin.display().to_string()
    };

    let service_content = format!(
        r#"[Unit]
        Description=Corn Stats
        After=network.target
        
        [Service]
        ExecStart={}
        Restart=always
        RestartSec=5
        
        [Install]
        WantedBy=multi-user.target
        "#,
        exec_path
    );

    fs::write(SERVICE_PATH, service_content).expect("Failed to write service file");
    Command::new("systemctl")
        .arg("daemon-reload")
        .status()
        .unwrap();

    Command::new("systemctl")
        .arg("enable")
        .arg("corn_stats.service")
        .status()
        .unwrap();

    println!("Corn Stats installed successfully.");
}

fn uninstall() {
    Command::new("systemctl")
        .arg("stop")
        .arg("corn_stats.service")
        .status()
        .ok();

    Command::new("systemctl")
        .arg("disable")
        .arg("corn_stats.service")
        .status()
        .ok();

    std::fs::remove_file(SERVICE_PATH).ok();

    Command::new("systemctl")
        .arg("daemon-reload")
        .status()
        .unwrap();

    println!("Corn Stats uninstalled.");
}

pub fn start() {
    Command::new("systemctl")
        .arg("start")
        .arg("corn_stats")
        .status()
        .unwrap();
}

pub fn stop() {
    Command::new("systemctl")
        .arg("stop")
        .arg("corn_stats")
        .status()
        .unwrap();
}
