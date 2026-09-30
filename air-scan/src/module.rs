#[derive(Default, Debug)]
pub struct Handshake {
    pub message1: HandshakeMSG,
    pub message2: HandshakeMSG,
    pub message3: HandshakeMSG,
    pub message4: HandshakeMSG,
}

#[derive(Default, Debug)]

pub struct HandshakeMSG {
    pub found: bool,
    pub packet: Vec<u8>,
}
