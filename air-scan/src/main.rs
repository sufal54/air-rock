mod module;
mod pcap;

use std::{
    collections::HashMap,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use airctl::{
    af_eth::{self, FrameName, PacketInfo, RadioInfo},
    helper::{CHANNEL_FREQ_MAP, channel_to_freq},
    nl_gen,
};

use ::terminal_manager::TerminalManager;
use clap::Parser;

use crate::{module::Handshake, pcap::PcapWriter};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Wireless interface
    #[arg(short, long)]
    interface: String,

    /// BSSID AP
    #[arg(short, long)]
    bssid: Option<String>,

    // Target Client
    #[arg(short, long)]
    target: Option<String>,

    /// Channel
    #[arg(short, long)]
    channel: Option<u16>,

    /// Pcap path
    #[arg(short = 'o', long)]
    handshake: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    let interface = cli.interface;
    let bssid = cli.bssid;
    let target = cli.target;
    let channel = cli.channel;
    let handshake = cli.handshake;

    let running = Arc::new(AtomicBool::new(true));
    let aps: Arc<RwLock<HashMap<String, (RadioInfo, PacketInfo)>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let connected_client: Arc<RwLock<HashMap<String, (RadioInfo, PacketInfo)>>> =
        Arc::new(RwLock::new(HashMap::new()));
    // let live_client: Arc<RwLock<HashMap<String, (RadioInfo, PacketInfo)>>> =
    //     Arc::new(RwLock::new(HashMap::new()));

    let terminal_aps: Arc<RwLock<HashMap<String, (RadioInfo, PacketInfo)>>> = Arc::clone(&aps);
    let terminal_connected: Arc<RwLock<HashMap<String, (RadioInfo, PacketInfo)>>> =
        Arc::clone(&connected_client);
    // let terminal_live: Arc<RwLock<HashMap<String, (RadioInfo, PacketInfo)>>> =
    //     Arc::clone(&live_client);
    let terminal_running = Arc::clone(&running);

    let worker_thread = thread::spawn(move || -> std::io::Result<()> {
        let mut terminal_mang = TerminalManager::new()?;

        terminal_mang.scaning(
            terminal_running,
            terminal_aps,
            terminal_connected,
            // terminal_live,
        )
    });

    let mut af_ctrl = af_eth::Ctrl::new(interface.clone());
    af_ctrl.sock_bind();
    let mut nl_ctrl = nl_gen::Ctrl::new(interface);
    nl_ctrl.bind();

    match (bssid, target, channel, handshake) {
        (None, None, None, None) => {
            let interface_info = match nl_ctrl.interface_info() {
                Some(iface) => iface,
                None => panic!("Interface not found"),
            };
            let max_freq = match interface_info.max_frequency {
                Some(freq) => freq,
                None => panic!("Uunknow Channel"),
            };

            let end = CHANNEL_FREQ_MAP.partition_point(|&(_, freq)| freq <= max_freq);

            let supported = &CHANNEL_FREQ_MAP[..end];

            while running.load(Ordering::Relaxed) {
                for &(_, freq) in supported {
                    // println!("Channel {channel}: {freq} MHz");
                    nl_ctrl.set_frequency(freq);
                    thread::sleep(Duration::from_millis(10));
                    af_ctrl.drain();

                    match af_ctrl.accept_timeout(200) {
                        Some((radio_info, packet_info, _)) => {
                            match packet_info.frame_name {
                                FrameName::Beacon => {
                                    let worker_aps = Arc::clone(&aps);

                                    let mut aps = worker_aps.write().unwrap();
                                    let key = packet_info.bssid.clone();
                                    aps.insert(key, (radio_info, packet_info));
                                }
                                // FrameName::ProbeRequest => {
                                //     let key = packet_info.transmitter.clone();

                                //     if is_multicast_mac(&key) {
                                //         continue;
                                //     }

                                //     let is_connected = {
                                //         let connected = connected_client.read().unwrap();
                                //         connected.contains_key(&key)
                                //     };

                                //     if !is_connected {
                                //         let mut live = live_client.write().unwrap();

                                //         live.insert(key, (radio_info, packet_info));
                                //     }
                                // }
                                FrameName::Data => {
                                    let worker_connected_client = Arc::clone(&connected_client);
                                    let mut connected_client =
                                        worker_connected_client.write().unwrap();

                                    let worker_aps = Arc::clone(&aps);

                                    let aps = worker_aps.read().unwrap();

                                    if !aps.contains_key(&packet_info.transmitter)
                                        || packet_info.transmitter == packet_info.receiver
                                    {
                                        continue;
                                    }

                                    // let worker_live_client = Arc::clone(&live_client);
                                    // let mut live_client = worker_live_client.write().unwrap();
                                    let key = packet_info.receiver.clone();

                                    // if live_client.contains_key(&key) {
                                    //     live_client.remove(&key);
                                    // }

                                    if is_multicast_mac(&key) {
                                        continue;
                                    }

                                    connected_client.insert(key, (radio_info, packet_info));
                                }
                                _ => {}
                            };
                        }
                        None => {}
                    }
                }
            }
        }

        (Some(bssid), None, Some(channel), None) => {
            let freq = match channel_to_freq(channel) {
                Some(freq) => freq,
                None => panic!("Unsupported channel: {channel}"),
            };

            nl_ctrl.set_frequency(freq);
            af_ctrl.drain();

            while running.load(Ordering::Relaxed) {
                // thread::sleep(Duration::from_millis(200));
                match af_ctrl.accept_timeout(200) {
                    Some((radio_info, packet_info, _)) => {
                        if packet_info.transmitter == bssid {
                            match packet_info.frame_name {
                                FrameName::Beacon => {
                                    let worker_aps = Arc::clone(&aps);

                                    let mut aps = worker_aps.write().unwrap();
                                    let key = packet_info.bssid.clone();
                                    aps.insert(key, (radio_info, packet_info));
                                }

                                FrameName::Data => {
                                    let worker_connected_client = Arc::clone(&connected_client);
                                    let mut connected_client =
                                        worker_connected_client.write().unwrap();

                                    let worker_aps = Arc::clone(&aps);

                                    let aps = worker_aps.read().unwrap();

                                    if !aps.contains_key(&packet_info.transmitter)
                                        || packet_info.transmitter == packet_info.receiver
                                    {
                                        continue;
                                    }

                                    let key = packet_info.receiver.clone();

                                    if is_multicast_mac(&key) {
                                        continue;
                                    }

                                    connected_client.insert(key, (radio_info, packet_info));
                                }
                                _ => {}
                            }
                        }
                    }
                    None => {}
                }
            }
        }

        (Some(bssid), None, Some(channel), Some(path)) => {
            let freq = match channel_to_freq(channel) {
                Some(freq) => freq,
                None => panic!("Unsupported channel: {channel}"),
            };

            let mut pcap_writer = match PcapWriter::new(&path) {
                Ok(pw) => pw,
                Err(_) => panic!("Unsupported path: {path}"),
            };

            nl_ctrl.set_frequency(freq);
            af_ctrl.drain();

            let mut targeted_client = String::new();
            let mut handshake = Handshake::default();
            let mut acceptd_beacon = false;

            while running.load(Ordering::Relaxed) {
                // thread::sleep(Duration::from_millis(100));

                match af_ctrl.accept_timeout(200) {
                    Some((radio_info, packet_info, raw_packet)) => {
                        if packet_info.transmitter != bssid && packet_info.receiver != bssid {
                            continue;
                        }

                        match packet_info.frame_name {
                            FrameName::Beacon => {
                                let worker_aps = Arc::clone(&aps);

                                let mut aps = worker_aps.write().unwrap();
                                let key = packet_info.bssid.clone();
                                aps.insert(key, (radio_info, packet_info));

                                if !acceptd_beacon {
                                    acceptd_beacon = true;
                                    let _ = pcap_writer.write_packet(&raw_packet);
                                }
                                continue;
                            }

                            FrameName::Data => {
                                let worker_connected_client = Arc::clone(&connected_client);
                                let mut connected_client = worker_connected_client.write().unwrap();

                                let worker_aps = Arc::clone(&aps);

                                let aps = worker_aps.read().unwrap();

                                if !aps.contains_key(&packet_info.transmitter)
                                    || packet_info.transmitter == packet_info.receiver
                                {
                                    continue;
                                }

                                let key = packet_info.receiver.clone();

                                if is_multicast_mac(&key) {
                                    continue;
                                }

                                connected_client.insert(key, (radio_info, packet_info));
                            }

                            FrameName::EapolKeyMessage1 => {
                                // AP -> Client
                                targeted_client = packet_info.receiver.clone();
                                handshake.message1.found = true;
                                handshake.message1.packet = raw_packet;

                                // pcap_writer.write_packet(&raw_packet).unwrap();

                                // println!("M1: {}", targeted_client);
                            }

                            FrameName::EapolKeyMessage2 => {
                                // Client -> AP
                                if packet_info.transmitter == targeted_client {
                                    handshake.message2.found = true;
                                    handshake.message2.packet = raw_packet;

                                    // pcap_writer.write_packet(&raw_packet).unwrap();

                                    // println!("M2: {}", targeted_client);
                                }
                            }

                            FrameName::EapolKeyMessage3 => {
                                // AP -> Client
                                if packet_info.receiver == targeted_client {
                                    handshake.message3.found = true;
                                    handshake.message3.packet = raw_packet;
                                    // pcap_writer.write_packet(&raw_packet).unwrap();

                                    // println!("M3: {}", targeted_client);
                                }
                            }

                            FrameName::EapolKeyMessage4 => {
                                // Client -> AP
                                if packet_info.transmitter == targeted_client {
                                    handshake.message4.found = true;
                                    handshake.message4.packet = raw_packet;
                                    // pcap_writer.write_packet(&raw_packet).unwrap();

                                    // println!("M4: {}", targeted_client);
                                }
                            }

                            _ => {}
                        }

                        if handshake.message1.found
                            && handshake.message2.found
                            && handshake.message3.found
                            && handshake.message4.found
                        {
                            let _ = pcap_writer.write_packet(&handshake.message1.packet);
                            let _ = pcap_writer.write_packet(&handshake.message2.packet);

                            let _ = pcap_writer.write_packet(&handshake.message3.packet);

                            let _ = pcap_writer.write_packet(&handshake.message4.packet);

                            running.store(false, Ordering::Relaxed);
                            println!("\n[+] Handshake captured successfully!");
                        }
                    }

                    None => {}
                }
            }
        }

        (Some(bssid), Some(target), Some(channel), Some(path)) => {
            let freq = match channel_to_freq(channel) {
                Some(freq) => freq,
                None => panic!("Unsupported channel: {channel}"),
            };

            let mut pcap_writer = match PcapWriter::new(&path) {
                Ok(pw) => pw,
                Err(_) => panic!("Unsupported path: {path}"),
            };

            nl_ctrl.set_frequency(freq);
            af_ctrl.drain();

            let mut targeted_client = String::new();
            let mut handshake = Handshake::default();
            let mut acceptd_beacon = false;

            while running.load(Ordering::Relaxed) {
                match af_ctrl.accept_timeout(200) {
                    Some((radio_info, packet_info, raw_packet)) => {
                        if !((packet_info.transmitter == bssid && packet_info.receiver == target)
                            || (packet_info.transmitter == target && packet_info.receiver == bssid))
                        {
                            continue;
                        }

                        match packet_info.frame_name {
                            FrameName::Beacon => {
                                let worker_aps = Arc::clone(&aps);

                                let mut aps = worker_aps.write().unwrap();
                                let key = packet_info.bssid.clone();
                                aps.insert(key, (radio_info, packet_info));

                                if !acceptd_beacon {
                                    acceptd_beacon = true;
                                    let _ = pcap_writer.write_packet(&raw_packet);
                                }
                                continue;
                            }

                            FrameName::Data => {
                                let worker_connected_client = Arc::clone(&connected_client);
                                let mut connected_client = worker_connected_client.write().unwrap();

                                let worker_aps = Arc::clone(&aps);

                                let aps = worker_aps.read().unwrap();

                                if !aps.contains_key(&packet_info.transmitter)
                                    || packet_info.transmitter == packet_info.receiver
                                {
                                    continue;
                                }

                                let key = packet_info.receiver.clone();

                                if is_multicast_mac(&key) {
                                    continue;
                                }

                                connected_client.insert(key, (radio_info, packet_info));
                            }

                            FrameName::EapolKeyMessage1 => {
                                // AP -> Client
                                targeted_client = packet_info.receiver.clone();
                                handshake.message1.found = true;
                                handshake.message1.packet = raw_packet;

                                handshake.message2.found = false;
                                handshake.message3.found = false;
                                handshake.message4.found = false;
                            }

                            FrameName::EapolKeyMessage2 => {
                                // Client -> AP
                                if packet_info.transmitter == targeted_client {
                                    handshake.message2.found = true;
                                    handshake.message2.packet = raw_packet;
                                }
                            }

                            FrameName::EapolKeyMessage3 => {
                                // AP -> Client
                                if packet_info.receiver == targeted_client {
                                    handshake.message3.found = true;
                                    handshake.message3.packet = raw_packet;
                                }
                            }

                            FrameName::EapolKeyMessage4 => {
                                // Client -> AP
                                if packet_info.transmitter == targeted_client {
                                    handshake.message4.found = true;
                                    handshake.message4.packet = raw_packet;
                                }
                            }

                            _ => {}
                        }

                        if handshake.message1.found
                            && handshake.message2.found
                            && handshake.message3.found
                            && handshake.message4.found
                        {
                            let _ = pcap_writer.write_packet(&handshake.message1.packet);
                            let _ = pcap_writer.write_packet(&handshake.message2.packet);

                            let _ = pcap_writer.write_packet(&handshake.message3.packet);

                            let _ = pcap_writer.write_packet(&handshake.message4.packet);

                            running.store(false, Ordering::Relaxed);
                            println!("\n[+] Handshake captured successfully!");
                        }
                    }

                    None => {}
                }
            }
        }
        _ => panic!("Unknow Commands"),
    }

    let _ = worker_thread.join().unwrap();
}

fn is_multicast_mac(mac: &str) -> bool {
    let first_byte = u8::from_str_radix(&mac[0..2], 16).unwrap_or(0);
    first_byte & 0x01 != 0
}
