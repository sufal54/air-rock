use std::{
    fs::File,
    io::{self, Write},
    time::{SystemTime, UNIX_EPOCH},
};

pub struct PcapWriter {
    file: File,
}

impl PcapWriter {
    pub fn new(path: &str) -> io::Result<Self> {
        let mut file = File::create(path)?;

        // PCAP Global Header
        file.write_all(&0xa1b2c3d4u32.to_le_bytes())?; // magic
        file.write_all(&2u16.to_le_bytes())?; // major
        file.write_all(&4u16.to_le_bytes())?; // minor
        file.write_all(&0i32.to_le_bytes())?; // thiszone
        file.write_all(&0u32.to_le_bytes())?; // sigfigs
        file.write_all(&65535u32.to_le_bytes())?; // snaplen

        // LINKTYPE_IEEE802_11_RADIOTAP
        file.write_all(&127u32.to_le_bytes())?;

        Ok(Self { file })
    }

    pub fn write_packet(&mut self, packet: &[u8]) -> io::Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        let ts_sec = now.as_secs() as u32;
        let ts_usec = now.subsec_micros();

        let len = packet.len() as u32;

        // PCAP packet header
        self.file.write_all(&ts_sec.to_le_bytes())?;
        self.file.write_all(&ts_usec.to_le_bytes())?;
        self.file.write_all(&len.to_le_bytes())?;
        self.file.write_all(&len.to_le_bytes())?;

        // Actual Radiotap + 802.11 packet
        self.file.write_all(packet)?;

        Ok(())
    }
}
