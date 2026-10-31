use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "air-rock")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Start { interface: String },
    Stop { interface: String },
    Down { interface: String },
    Up { interface: String },
    Info { interface: String },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { interface } => {
            println!();
            println!("┌──────────────────────────────────────┐");
            println!("│          Starting Monitor Mode       │");
            println!("└──────────────────────────────────────┘");
            println!();
            println!("  Interface : {}", interface);

            let mut gen_ctrl = airctl::nl_gen::Ctrl::new(interface.clone());
            let mut rout_ctrl = airctl::nl_route::Ctrl::new(interface.clone());

            println!("  [1/4] Preparing interface...");
            std::thread::sleep(std::time::Duration::from_millis(0x1f4));

            rout_ctrl.set_interface_power(false);

            println!("  [2/4] Interface powered off");
            std::thread::sleep(std::time::Duration::from_millis(0x1f4));

            gen_ctrl.bind();

            println!("  [3/4] Switching to monitor mode...");

            if gen_ctrl.start_monitor() {
                println!("        ✓ Monitor mode enabled");
            } else {
                println!("        ✗ Failed to enable monitor mode");
                println!();
                println!("✗ Failed to start monitor mode on {}", interface);
                println!();

                println!("  [4/4] Interface powered on...");
                std::thread::sleep(std::time::Duration::from_millis(0x1f4));

                rout_ctrl.set_interface_power(true);

                println!();
                println!("✓ Monitor mode failed {}", interface);
                println!();

                return;
            }

            std::thread::sleep(std::time::Duration::from_millis(0x1f4));

            rout_ctrl.set_interface_power(true);

            println!("  [4/4] Interface powered on");
            std::thread::sleep(std::time::Duration::from_millis(0x1f4));

            println!();
            println!("✓ Monitor mode started on {}", interface);
            println!();
        }
        Commands::Stop { interface } => {
            println!();
            println!("┌──────────────────────────────────────┐");
            println!("│          Stopping Monitor Mode       │");
            println!("└──────────────────────────────────────┘");
            println!();
            println!("  Interface : {}", interface);

            let mut gen_ctrl = airctl::nl_gen::Ctrl::new(interface.clone());

            std::thread::sleep(std::time::Duration::from_millis(0x1f4));

            let mut rout_ctrl = airctl::nl_route::Ctrl::new(interface.clone());

            println!("  [1/4] Preparing interface...");
            std::thread::sleep(std::time::Duration::from_millis(0x1f4));

            rout_ctrl.set_interface_power(false);

            println!("  [2/4] Interface powered off");
            std::thread::sleep(std::time::Duration::from_millis(0x1f4));
            gen_ctrl.bind();

            println!("  [3/4] Switching to managed mode...");
            std::thread::sleep(std::time::Duration::from_millis(0x1f4));
            if gen_ctrl.stop_monitor() {
                println!("        ✓ Managed mode enabled");
            } else {
                println!("        ✗ Failed to disable monitor mode");
                println!();

                println!("  [4/4] Interface powered on...");
                std::thread::sleep(std::time::Duration::from_millis(0x1f4));

                rout_ctrl.set_interface_power(true);

                println!();
                println!("✗ Failed to stop monitor mode on {}", interface);
                println!();

                return;
            }

            std::thread::sleep(std::time::Duration::from_millis(0x1f4));
            rout_ctrl.set_interface_power(true);
            println!("  [4/4] Interface powered on");

            std::thread::sleep(std::time::Duration::from_millis(0x1f4));

            println!();
            println!("  Scanning for interface...");
            for attempt in 0..10 {
                if gen_ctrl.scan() {
                    println!("        ✓ Interface detected");
                    break;
                }
                println!("        Waiting... ({}/10)", attempt);
                std::thread::sleep(std::time::Duration::from_millis(0x1f4));
            }
        }

        Commands::Down { interface } => {
            println!();
            println!("┌──────────────────────────────────────┐");
            println!("│           Disabling Interface        │");
            println!("└──────────────────────────────────────┘");
            println!();
            println!("  Interface : {}", interface);

            let mut rout_ctrl = airctl::nl_route::Ctrl::new(interface.clone());
            println!("  [1/1] Powering off interface...");
            std::thread::sleep(std::time::Duration::from_millis(0x1f4));

            rout_ctrl.set_interface_power(false);

            println!("        ✓ Interface powered off");
            println!();
            println!("✓ Interface disabled: {}", interface);
            println!();
        }
        Commands::Up { interface } => {
            println!();
            println!("┌──────────────────────────────────────┐");
            println!("│            Enabling Interface        │");
            println!("└──────────────────────────────────────┘");
            println!();
            println!("  Interface : {}", interface);

            let mut gen_ctrl = airctl::nl_gen::Ctrl::new(interface.clone());

            println!("  [1/3] Preparing interface...");
            std::thread::sleep(std::time::Duration::from_millis(0x1f4));

            let mut rout_ctrl = airctl::nl_route::Ctrl::new(interface.clone());
            std::thread::sleep(std::time::Duration::from_millis(0x1f4));
            gen_ctrl.bind();

            println!("  [2/3] Powering on interface...");

            std::thread::sleep(std::time::Duration::from_millis(0x1f4));

            rout_ctrl.set_interface_power(true);

            println!("        ✓ Interface powered on");

            std::thread::sleep(std::time::Duration::from_millis(0x1f4));

            for attempt in 0..10 {
                if gen_ctrl.scan() {
                    println!("        ✓ Interface detected");
                    break;
                }
                println!("        Waiting... ({}/10)", attempt);
                std::thread::sleep(std::time::Duration::from_millis(0x1f4));
            }
        }
        Commands::Info { interface } => {
            println!();
            println!("┌──────────────────────────────────────┐");
            println!("│            Interface Info            │");
            println!("└──────────────────────────────────────┘");
            println!();
            println!("  Interface : {}", interface);

            let mut ctrl = airctl::nl_gen::Ctrl::new(interface);

            println!("  [1/2] Connecting to interface...");
            std::thread::sleep(std::time::Duration::from_millis(0x1f4));

            ctrl.bind();
            println!("        ✓ Connected");
            println!("  [2/2] Reading interface information...");
            std::thread::sleep(std::time::Duration::from_millis(0x1f4));

            match ctrl.interface_info() {
                Some(info) => {
                    println!();
                    println!("  Interface");
                    println!("  ─────────────────────────────────────");

                    println!("  Name           : {:?}", info.ifname);
                    println!("  Index          : {:?}", info.ifindex);
                    println!("  Type           : {:?}", info.iftype);
                    println!("  MAC            : {:?}", info.mac);

                    println!();
                    println!("  Wireless");
                    println!("  ─────────────────────────────────────");

                    println!("  Wiphy          : {:?}", info.wiphy);
                    println!("  Wdev           : {:?}", info.wdev);
                    println!("  Connected      : {}", info.connected);
                    println!("  BSSID          : {:?}", info.bssid);
                    println!("  SSID           : {:?}", info.ssid);

                    println!();
                    println!("  Channel");
                    println!("  ─────────────────────────────────────");

                    println!("  Frequency      : {:?} MHz", info.frequency);
                    println!("  Max Frequency  : {:?} MHz", info.max_frequency);
                    println!("  Channel        : {:?}", info.channel);
                    println!("  Channel Width  : {:?} MHz", info.channel_width);

                    // println!();
                    // println!("  Signal");
                    // println!("  ─────────────────────────────────────");

                    // println!("  Signal         : {:?} dBm", info.signal_dbm);
                    // println!("  Signal Average : {:?} dBm", info.signal_avg_dbm);

                    // println!();
                    // println!("  Network");
                    // println!("  ─────────────────────────────────────");

                    // println!("  MTU            : {:?}", info.mtu);
                    // println!("  VIF Radio Mask : {:?}", info.vif_radio_mask);
                    // println!("  Generation     : {:?}", info.generation);
                    // println!("  Four Address   : {}", info.four_addr);
                }

                None => {
                    println!();
                    println!("✗ Failed to get interface information");
                }
            }
        }
    }
}
