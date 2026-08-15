use std::{
    ffi::{CString, c_void},
    io, mem, thread,
    time::Duration,
};

use airctl::{af_eth, helper::CHANNEL_FREQ_MAP, nl_gen};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Wireless interface
    #[arg(short, long, default_value = "wlan0")]
    interface: String,

    /// BSSID
    #[arg(short, long)]
    bssid: Option<String>,

    /// Channel
    #[arg(short, long)]
    channel: Option<u8>,
}

fn main() {
    let cli = Cli::parse();

    let interface = cli.interface;
    let bssid = cli.bssid;
    let channel = cli.channel;

    let mut af_ctrl = af_eth::Ctrl::new(interface.clone());
    af_ctrl.sock_bind();
    let mut nl_ctrl = nl_gen::Ctrl::new(interface);
    nl_ctrl.bind();

    match (bssid, channel) {
        (None, None) => {
            let interface_info = match nl_ctrl.interface_info() {
                Some(iface) => iface,
                None => panic!(),
            };
            let max_freq = match interface_info.max_frequency {
                Some(freq) => freq,
                None => panic!(),
            };
            println!("{}", max_freq);

            let end = CHANNEL_FREQ_MAP.partition_point(|&(_, freq)| freq <= max_freq);

            let supported = &CHANNEL_FREQ_MAP[..end];

            loop {
                for &(channel, freq) in supported {
                    println!("Channel {channel}: {freq} MHz");
                    nl_ctrl.set_frequency(freq);

                    thread::sleep(Duration::from_millis(20));
                    af_ctrl.accept_timeout(200);
                }
            }
        }

        (Some(_), Some(_)) => todo!(),
        (_) => panic!(),
    }
}
