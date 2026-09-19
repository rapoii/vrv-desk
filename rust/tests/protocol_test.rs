use mirror_core::protocol::{InputEvent, MouseButton, WirePacket};

#[test]
fn test_wire_packet_serialization_roundtrip() {
    let input = WirePacket::Input(InputEvent::MouseDown {
        x: 0.45,
        y: 0.82,
        button: MouseButton::Left,
    });

    let bytes = input.serialize();
    let decoded = WirePacket::deserialize(&bytes).expect("Valid decode");

    match decoded {
        WirePacket::Input(InputEvent::MouseDown { x, y, button }) => {
            assert!((x - 0.45).abs() < 1e-5);
            assert!((y - 0.82).abs() < 1e-5);
            assert_eq!(button, MouseButton::Left);
        }
        _ => panic!("Expected Input packet"),
    }
}
