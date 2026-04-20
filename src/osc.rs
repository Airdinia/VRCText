use std::io;
use std::net::{SocketAddr, UdpSocket};

fn pad_to_4(v: &mut Vec<u8>) {
    while v.len() % 4 != 0 {
        v.push(0);
    }
}

fn write_osc_string(v: &mut Vec<u8>, s: &str) {
    v.extend_from_slice(s.as_bytes());
    v.push(0);
    pad_to_4(v);
}

pub fn encode_chatbox_input(msg: &str, bypass: bool, sound: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(64 + msg.len());
    write_osc_string(&mut out, "/chatbox/input");
    let tag = format!(",s{}{}", if bypass { 'T' } else { 'F' }, if sound { 'T' } else { 'F' });
    write_osc_string(&mut out, &tag);
    write_osc_string(&mut out, msg);
    out
}

pub fn encode_chatbox_typing(typing: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(32);
    write_osc_string(&mut out, "/chatbox/typing");
    let tag = if typing { ",T" } else { ",F" };
    write_osc_string(&mut out, tag);
    out
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
