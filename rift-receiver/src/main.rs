use clap::Parser;
use evdev::uinput::VirtualDeviceBuilder;
use evdev::{AttributeSet, InputEvent, EventType, Key, RelativeAxisType};
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;

#[derive(Parser, Debug)]
#[command(version, about = "Rift Receiver: The Virtual Mouse")]
struct Args {
    /// Port to listen on (e.g., 9000)
    #[arg(short, long, default_value = "9000")]
    port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
enum Packet {
    Event { e_type: u16, code: u16, value: i32 },
    ConfigRequest,
    ConfigResponse { width: i32 },
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // 0. Detect Screen Width
    let screen_width = display_info::DisplayInfo::all()
        .map(|displays| {
            displays.iter()
                .find(|d| d.is_primary)
                .map(|d| d.width as i32)
                .unwrap_or(1920)
        })
        .unwrap_or(1920);

    println!("🖥️  Local Screen Width: {}", screen_width);

    // 1. Configure the Virtual Device
    let mut keys = AttributeSet::<Key>::new();
    for i in 0..512 {
        keys.insert(Key::new(i));
    }

    let mut rels = AttributeSet::<RelativeAxisType>::new();
    rels.insert(RelativeAxisType::REL_X);
    rels.insert(RelativeAxisType::REL_Y);
    rels.insert(RelativeAxisType::REL_WHEEL);
    rels.insert(RelativeAxisType::REL_HWHEEL);

    let mut device = VirtualDeviceBuilder::new()?
        .name("Rift Virtual Mouse")
        .with_keys(&keys)?
        .with_relative_axes(&rels)?
        .build()
        .expect("Failed to create virtual device. Do you have permission on /dev/uinput?");

    println!("👻 Rift Receiver Active. Device created: Rift Virtual Mouse");

    // 2. Setup Network
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", args.port))?;
    println!("👂 Listening on UDP port {}", args.port);

    // 3. The Loop
    let mut buf = [0u8; 1024];
    loop {
        let (amt, src) = socket.recv_from(&mut buf)?;
        
        if let Ok(packet) = bincode::deserialize::<Packet>(&buf[..amt]) {
            match packet {
                Packet::Event { e_type, code, value } => {
                    let event = InputEvent::new(EventType(e_type), code, value);
                    device.emit(&[event])?;
                }
                Packet::ConfigRequest => {
                    println!("🤝 Handshake requested from {}", src);
                    let response = Packet::ConfigResponse { width: screen_width };
                    let encoded = bincode::serialize(&response).unwrap();
                    socket.send_to(&encoded, src)?;
                }
                _ => {}
            }
        }
    }
}
