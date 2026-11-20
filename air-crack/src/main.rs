use clap::{Parser, Subcommand};
use hmac::{Hmac, Mac};
use pbkdf2::pbkdf2_hmac;
use pcap_file::pcap::PcapReader;
use sha1::Sha1;
use sha2::Sha256;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    ops::ControlFlow,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};
use terminal_manager::TerminalManager;

#[derive(Default)]
enum Encryption {
    SHA1,
    SHA2,
    FT_PSK,
    #[default]
    Unknown,
}

#[derive(Default)]
struct Info {
    pub ap_ssid: String,
    pub ap_bssid: [u8; 6],
    pub client_bssid: [u8; 6],
    pub a_nonce: [u8; 32],
    pub s_nonce: [u8; 32],
    pub mic: String,
    pub encryption: Encryption,
    pub eapol_frame: Vec<u8>,
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Pcap path
    #[arg(short, long)]
    pcap: String,

    #[command(subcommand)]
    mode: Mode,
}

#[derive(Subcommand, Debug)]
enum Mode {
    #[command(name = "W")]
    Wordlist {
        #[arg(short, long)]
        wordlist_path: String,
    },
    #[command(name = "B")]
    Bruteforce {
        #[arg(short, long)]
        keys: String,

        #[arg(short, long)]
        low: usize,
        #[arg(short, long)]
        max: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    let pcap_path = cli.pcap;

    let pcap_file = match File::open(pcap_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("File error: {}", e);
            return;
        }
    };

    let mut reader = match PcapReader::new(pcap_file) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("PCAP error: {}", e);
            return;
        }
    };

    let mut info = Info::default();

    while let Some(packet) = reader.next_packet() {
        let packet = match packet {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Read error: {}", e);
                continue;
            }
        };

        let data = &packet.data;

        if data.len() < 8 {
            continue;
        }

        let rt_len = u16::from_le_bytes([data[2], data[3]]) as usize;

        if rt_len < 8 || rt_len > data.len() {
            continue;
        }

        let wifi = &data[rt_len..];

        if wifi.len() < 24 {
            continue;
        }

        let fc = u16::from_le_bytes([wifi[0], wifi[1]]);

        let frame_type = (fc >> 2) & 3;
        let subtype = (fc >> 4) & 15;

        if frame_type == 0 && subtype == 8 {
            if wifi.len() < 36 {
                panic!("Beacon fixed fields missing");
            }

            parse_beacon(&mut info, &wifi);
        } else if frame_type == 2 {
            let to_ds = fc & 0x0100 != 0;
            let from_ds = fc & 0x0200 != 0;

            let mut header_len = 24;

            if to_ds && from_ds {
                header_len += 6;
            }

            if subtype & 8 != 0 {
                header_len += 2;
            }

            if fc & 0x8000 != 0 && subtype & 8 != 0 {
                header_len += 4;
            }

            if wifi.len() >= header_len + 12 {
                let llc = &wifi[header_len..];

                if llc.starts_with(&[0xaa, 0xaa, 0x03, 0x00, 0x00, 0x00, 0x88, 0x8e]) {
                    let eapol = &llc[8..];

                    if eapol.len() >= 7 && eapol[1] == 3 {
                        let key_info = u16::from_be_bytes([eapol[5], eapol[6]]);

                        let ack = key_info & (1 << 7) != 0;
                        let mic = key_info & (1 << 8) != 0;
                        let install = key_info & (1 << 6) != 0;
                        let secure = key_info & (1 << 9) != 0;

                        match (ack, mic, install, secure) {
                            (true, false, false, false) => {
                                info.ap_bssid.copy_from_slice(&wifi[10..16]);
                                info.client_bssid.copy_from_slice(&wifi[4..10]);

                                let anonce = &eapol[17..49];

                                info.a_nonce.copy_from_slice(anonce);
                            }
                            (false, true, false, false) => {
                                info.eapol_frame = eapol.to_vec();
                                let snonce = &eapol[17..49];
                                let mic = &eapol[81..97];

                                info.s_nonce.copy_from_slice(snonce);

                                info.mic =
                                    mic.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                            }
                            // (true, true, true, true) => {
                            //     println!("Message 3")
                            // }
                            // (false, true, false, true) => {
                            //     println!("Message 4")
                            // }
                            _ => {}
                        };
                    }
                }
            }
        }
    }

    let running = Arc::new(AtomicBool::new(true));
    let terminal_running = Arc::clone(&running);

    let is_password_found = Arc::new(AtomicBool::new(false));
    let password_status = Arc::clone(&is_password_found);

    let founded_password = Arc::new(RwLock::new(None::<String>));
    let password_clone = Arc::clone(&founded_password);

    let terminal_key = Arc::new(RwLock::new(String::new()));
    let terminal_kck = Arc::new(RwLock::new(String::new()));

    let key_clone = Arc::clone(&terminal_key);
    let kck_clone = Arc::clone(&terminal_kck);

    let worker_thread = thread::spawn(move || -> std::io::Result<()> {
        let mut terminal_mang = TerminalManager::new()?;

        terminal_mang.cracking(
            terminal_running,
            password_status,
            password_clone,
            key_clone,
            kck_clone,
        )
    });
    match cli.mode {
        Mode::Wordlist { wordlist_path } => {
            let wordlist_file = File::open(wordlist_path).expect("Wordlist path should be vaild!");
            let reader = BufReader::new(wordlist_file);

            for line in reader.lines() {
                if !running.load(Ordering::Relaxed) {
                    break;
                }

                if let Ok(word) = line {
                    *terminal_key.write().unwrap() = word.clone();

                    let pmk = derive_pmk(word.as_bytes(), info.ap_ssid.as_bytes());
                    let ptk = derive_ptk(
                        &info.encryption,
                        &pmk,
                        &info.ap_bssid,
                        &info.client_bssid,
                        &info.a_nonce,
                        &info.s_nonce,
                    );
                    let mut kck = [0u8; 16];
                    kck.copy_from_slice(&ptk[..16]);

                    *terminal_kck.write().unwrap() =
                        kck.iter().map(|b| format!("{:02X}", b)).collect::<String>();

                    if verify_eapol_mic(&info.encryption, &kck, &info.eapol_frame) {
                        *founded_password.write().unwrap() = Some(word);
                        is_password_found.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            }
            running.store(false, Ordering::Relaxed);
        }

        Mode::Bruteforce { keys, low, max } => {
            generate_combinations(keys.as_bytes(), low, max, |combination| {
                if !running.load(Ordering::Relaxed) {
                    return ControlFlow::Break(());
                }
                let s: &str = std::str::from_utf8(combination).unwrap();

                *terminal_key.write().unwrap() = s.to_string();

                let pmk = derive_pmk(combination, info.ap_ssid.as_bytes());
                let ptk = derive_ptk(
                    &info.encryption,
                    &pmk,
                    &info.ap_bssid,
                    &info.client_bssid,
                    &info.a_nonce,
                    &info.s_nonce,
                );
                let mut kck = [0u8; 16];
                kck.copy_from_slice(&ptk[..16]);

                *terminal_kck.write().unwrap() =
                    kck.iter().map(|b| format!("{:02X}", b)).collect::<String>();

                if verify_eapol_mic(&info.encryption, &kck, &info.eapol_frame) {
                    *founded_password.write().unwrap() = Some(s.to_string());

                    is_password_found.store(true, Ordering::Relaxed);

                    return ControlFlow::Break(());
                }
                ControlFlow::Continue(())
            });

            if running.load(Ordering::Relaxed) {
                running.store(false, Ordering::Relaxed);
            }
        }
    }

    let _ = worker_thread.join().unwrap();
}

fn generate_combinations<F>(letters: &[u8], min_len: usize, max_len: usize, mut callback: F)
where
    F: FnMut(&[u8]) -> ControlFlow<()>,
{
    if letters.is_empty() || min_len == 0 || min_len > max_len {
        return;
    }

    for len in min_len..=max_len {
        let mut indices = vec![0usize; len];
        let mut word = vec![0u8; len];

        loop {
            for i in 0..len {
                word[i] = letters[indices[i]];
            }

            if let ControlFlow::Break(()) = callback(&word) {
                return;
            }

            let mut pos = len;

            while pos > 0 {
                pos -= 1;
                indices[pos] += 1;

                if indices[pos] < letters.len() {
                    break;
                }

                indices[pos] = 0;
            }

            if pos == 0 && indices[0] == 0 {
                break;
            }
        }
    }
}

fn parse_beacon(info: &mut Info, packet: &[u8]) {
    let mut pos = 24;

    if packet.len() < 12 {
        panic!("Beacon body too short: {}", packet.len());
    }

    pos += 12;

    while pos + 2 <= packet.len() {
        let id = packet[pos];
        let len = packet[pos + 1] as usize;
        pos += 2;

        if len > packet.len() - pos {
            panic!("Invalid IE id: {}", id);
        }

        let value = &packet[pos..pos + len];

        match id {
            0 => {
                if len == 0 {
                    panic!("SSID: [Hidden / Broadcast SSID]");
                } else {
                    let ssid = String::from_utf8_lossy(value);
                    info.ap_ssid = ssid.to_string();
                }
            }

            48 => {
                if len < 8 {
                    continue;
                }

                let mut p = 0;

                // Version
                p += 2;

                // Group Cipher Suite
                p += 4;

                // Pairwise Cipher Suite count
                let pairwise_count = u16::from_le_bytes([value[p], value[p + 1]]) as usize;
                p += 2;

                // Pairwise Cipher Suites
                p += pairwise_count * 4;

                if p + 2 > len {
                    continue;
                }

                // AKM Suite count
                let akm_count = u16::from_le_bytes([value[p], value[p + 1]]) as usize;
                p += 2;

                if p + akm_count * 4 > len {
                    continue;
                }

                for _ in 0..akm_count {
                    let suite = &value[p..p + 4];

                    match suite {
                        [0x00, 0x0f, 0xac, 0x02] => {
                            info.encryption = Encryption::SHA1;
                        }
                        [0x00, 0x0f, 0xac, 0x06] => {
                            info.encryption = Encryption::SHA2;
                        }
                        [0x00, 0x0f, 0xac, 0x04] => {
                            info.encryption = Encryption::FT_PSK;
                        }
                        _ => {}
                    }

                    p += 4;
                }
            }
            _ => {}
        }

        pos += len;
    }
}

fn verify_eapol_mic(encoded: &Encryption, kck: &[u8; 16], eapol_frame: &[u8]) -> bool {
    let mic_offset = 81;
    let mut modified_frame = eapol_frame.to_vec();

    if modified_frame.len() < mic_offset + 16 {
        return false;
    }

    for i in 0..16 {
        modified_frame[mic_offset + i] = 0x00;
    }

    let result = match encoded {
        &Encryption::SHA1 => {
            let mut mac =
                Hmac::<Sha1>::new_from_slice(kck).expect("KCK size is valid for HMAC-SHA1");

            mac.update(&modified_frame);
            mac.finalize().into_bytes().to_vec()
        }
        &Encryption::SHA2 => {
            let mut mac =
                Hmac::<Sha256>::new_from_slice(kck).expect("KCK size is valid for HMAC-SHA256");

            mac.update(&modified_frame);
            mac.finalize().into_bytes().to_vec()
        }
        _ => panic!("Unsupported encryption"),
    };

    let computed_mic = &result[..16];

    let received_mic = &eapol_frame[mic_offset..mic_offset + 16];

    subtle_compare(computed_mic, received_mic)
}

fn subtle_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn derive_ptk(
    encoded: &Encryption,
    pmk: &[u8],
    aa: &[u8; 6],
    sa: &[u8; 6],
    anonce: &[u8; 32],
    snonce: &[u8; 32],
) -> [u8; 48] {
    let label = b"Pairwise key expansion";

    let (min_mac, max_mac) = if aa < sa { (aa, sa) } else { (sa, aa) };

    let (min_nonce, max_nonce) = if anonce < snonce {
        (anonce, snonce)
    } else {
        (snonce, anonce)
    };

    let mut data = Vec::with_capacity(label.len() + 1 + 12 + 64 + 1);
    data.extend_from_slice(label);
    data.push(0x00);
    data.extend_from_slice(min_mac);
    data.extend_from_slice(max_mac);
    data.extend_from_slice(min_nonce);
    data.extend_from_slice(max_nonce);

    let base_len = data.len();
    data.push(0x00);

    let mut ptk = [0u8; 48];
    let iterations = (ptk.len() + 19) / 20;

    for i in 0..iterations {
        data[base_len] = i as u8;

        let result = match encoded {
            &Encryption::SHA1 => {
                let mut mac =
                    Hmac::<Sha1>::new_from_slice(pmk).expect("HMAC accepts keys of any size");

                mac.update(&data);
                mac.finalize().into_bytes().to_vec()
            }
            &Encryption::SHA2 => {
                let mut mac =
                    Hmac::<Sha256>::new_from_slice(pmk).expect("HMAC accepts keys of any size");

                mac.update(&data);
                mac.finalize().into_bytes().to_vec()
            }
            _ => panic!("Unsupported encryption"),
        };

        let hash_len = result.len();
        let start = i * hash_len;
        let copy_len = std::cmp::min(hash_len, ptk.len() - start);

        ptk[start..start + copy_len].copy_from_slice(&result[..copy_len]);
    }

    ptk
}

fn derive_pmk(password: &[u8], ssid: &[u8]) -> [u8; 32] {
    let mut pmk = [0u8; 32];

    pbkdf2_hmac::<Sha1>(password, ssid, 4096, &mut pmk);

    pmk
}
