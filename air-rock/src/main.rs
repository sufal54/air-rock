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
            println!("Start monitor mode on {}", interface);
            let mut gen_ctrl = airctl::nl_gen::Ctrl::new(interface.clone());
            let mut rout_ctrl = airctl::nl_route::Ctrl::new(interface);
            std::thread::sleep(std::time::Duration::from_millis(500));

            rout_ctrl.set_interface_power(false);
            std::thread::sleep(std::time::Duration::from_millis(500));
            gen_ctrl.bind();
            println!("{}", gen_ctrl.start_monitor());
            std::thread::sleep(std::time::Duration::from_millis(500));

            rout_ctrl.set_interface_power(true);

            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        Commands::Stop { interface } => {
            println!("Stop monitor mode on {}", interface);
            let mut gen_ctrl = airctl::nl_gen::Ctrl::new(interface.clone());
            std::thread::sleep(std::time::Duration::from_millis(500));

            let mut rout_ctrl = airctl::nl_route::Ctrl::new(interface);
            std::thread::sleep(std::time::Duration::from_millis(500));

            rout_ctrl.set_interface_power(false);
            std::thread::sleep(std::time::Duration::from_millis(500));
            gen_ctrl.bind();
            std::thread::sleep(std::time::Duration::from_millis(500));
            println!("{}", gen_ctrl.stop_monitor());
            std::thread::sleep(std::time::Duration::from_millis(500));

            rout_ctrl.set_interface_power(true);

            std::thread::sleep(std::time::Duration::from_millis(500));

            for _ in 0..10 {
                if gen_ctrl.scan() {
                    break;
                }

                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }

        Commands::Down { interface } => {
            let mut rout_ctrl = airctl::nl_route::Ctrl::new(interface);
            std::thread::sleep(std::time::Duration::from_millis(500));

            rout_ctrl.set_interface_power(false);
        }
        Commands::Up { interface } => {
            let mut gen_ctrl = airctl::nl_gen::Ctrl::new(interface.clone());
            std::thread::sleep(std::time::Duration::from_millis(500));

            let mut rout_ctrl = airctl::nl_route::Ctrl::new(interface);
            std::thread::sleep(std::time::Duration::from_millis(500));
            gen_ctrl.bind();
            std::thread::sleep(std::time::Duration::from_millis(500));

            rout_ctrl.set_interface_power(true);

            std::thread::sleep(std::time::Duration::from_millis(500));

            for _ in 0..10 {
                if gen_ctrl.scan() {
                    break;
                }

                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
        Commands::Info { interface } => {
            let mut ctrl = airctl::nl_gen::Ctrl::new(interface);
            std::thread::sleep(std::time::Duration::from_millis(500));

            ctrl.bind();
            std::thread::sleep(std::time::Duration::from_millis(500));

            println!("Interface Info {:#?}", ctrl.interface_info());
        }
    }
}
