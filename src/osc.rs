use rosc::{OscMessage, OscPacket, OscType};
use std::io;
use std::net::{SocketAddr, UdpSocket};

pub fn encode_chatbox_input(msg: &str, bypass: bool, sound: bool) -> Vec<u8> {
    let pkt = OscPacket::Message(OscMessage {
        addr: "/chatbox/input".into(),
        args: vec![
            OscType::String(msg.into()),
            OscType::Bool(bypass),
            OscType::Bool(sound),
        ],
    });
    rosc::encoder::encode(&pkt).unwrap_or_default()
}

pub fn encode_chatbox_typing(typing: bool) -> Vec<u8> {
    let pkt = OscPacket::Message(OscMessage {
        addr: "/chatbox/typing".into(),
        args: vec![OscType::Bool(typing)],
    });
    rosc::encoder::encode(&pkt).unwrap_or_default()
}

pub fn send(socket: &UdpSocket, target: SocketAddr, packet: &[u8]) -> io::Result<()> {
    socket.send_to(packet, target).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_packet_is_4_aligned() {
        let p = encode_chatbox_input("hi", true, true);
        assert_eq!(p.len() % 4, 0);
        assert!(p.starts_with(b"/chatbox/input\0"));
    }

    #[test]
    fn typing_tag_reflects_bool() {
        let t = encode_chatbox_typing(true);
        let f = encode_chatbox_typing(false);
        assert!(t.windows(2).any(|w| w == b",T"));
        assert!(f.windows(2).any(|w| w == b",F"));
    }
}
