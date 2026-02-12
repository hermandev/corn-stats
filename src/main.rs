use std::{collections::HashMap, fs, path::PathBuf, time::Duration};

use appindicator3::{Indicator, traits::AppIndicatorExt};
use gtk::prelude::*;
use sysinfo::{Networks, System};

fn main() {
    create_autostart();
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

fn create_autostart() {
    let home = std::env::var("HOME").expect("Cannot get HOME directory");
    let mut path = PathBuf::from(home);
    path.push(".config/autostart");

    // Buat folder kalau belum ada
    fs::create_dir_all(&path).ok();

    let current_bin = std::env::current_exe().unwrap();

    let desktop_entry = format!(
        r#"[Desktop Entry]
        Type=Application
        Exec={}
        Hidden=false
        NoDisplay=false
        X-GNOME-Autostart-enabled=true
        Name=Corn Stats
        Comment=System tray monitoring
        "#,
        current_bin.display()
    );

    path.push("corn_stats.desktop");

    if !path.exists() {
        fs::write(&path, desktop_entry).expect("Failed to create autostart file");
        println!("Autostart enabled.");
    }
}
