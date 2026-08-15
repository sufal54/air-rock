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
    Info { interface: String },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { interface } => {
            println!("Start monitor mode on {}", interface);
            let mut ctrl = airctl::nl_gen::Ctrl::new(interface);
            ctrl.bind();
            println!("{}", ctrl.start_monitor());
        }
        Commands::Stop { interface } => {
            println!("Start monitor mode on {}", interface);
            let mut ctrl = airctl::nl_gen::Ctrl::new(interface);
            ctrl.bind();
            println!("{}", ctrl.stop_monitor());
        }
        Commands::Info { interface } => {
            let mut ctrl = airctl::nl_gen::Ctrl::new(interface);
            ctrl.bind();
            println!("Interface Info {:#?}", ctrl.interface_info());
        }
    }
}
