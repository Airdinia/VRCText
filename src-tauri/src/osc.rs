use rosc::{OscMessage, OscPacket, OscType};
use std::io;
use std::net::{SocketAddr, UdpSocket};

pub const MAX_CHATBOX_CHARS: usize = 144;

pub fn validate_message(text: &str) -> Result<&str, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("@i18n:errEmptyMessage".into());
    }
    if text.contains('\0') || text.chars().count() > MAX_CHATBOX_CHARS {
        return Err("@i18n:errInvalidMessage".into());
    }
    Ok(text)
}

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
    fn message_limits_apply_to_unicode_and_reject_nul() {
        assert!(validate_message("  ").is_err());
        assert!(validate_message("hello\0world").is_err());
        assert!(validate_message(&"你".repeat(144)).is_ok());
        assert!(validate_message(&"😀".repeat(145)).is_err());
        assert_eq!(validate_message("  hello  ").unwrap(), "hello");
    }

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
