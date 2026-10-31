use std::{
    fs::File,
    io,
    time::{SystemTime, UNIX_EPOCH},
};

use pcap_file::pcap::{PcapHeader, PcapPacket, PcapWriter};

pub struct PcapManager {
    writer: PcapWriter<File>,
}

impl PcapManager {
    pub fn new(path: &str) -> io::Result<Self> {
        let file = File::create(path)?;

        let pcap_header = PcapHeader {
            datalink: pcap_file::DataLink::IEEE802_11_RADIOTAP,
            ts_resolution: pcap_file::TsResolution::MicroSecond,
            snaplen: 65535,
            ..Default::default()
        };
        let writer = PcapWriter::with_header(file, pcap_header)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(Self { writer })
    }

    pub fn append_packet(&mut self, packet: &[u8]) -> io::Result<()> {
        if packet.is_empty() {
            return Ok(());
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        let pcap_packet = PcapPacket {
            timestamp: now,
            orig_len: packet.len() as u32,
            data: packet.into(),
        };

        self.writer
            .write_packet(&pcap_packet)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(())
    }
}
