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

// Same wire format as Sender
#[derive(Serialize, Deserialize, Debug)]
struct InputPacket {
    e_type: u16,
    code: u16,
    value: i32,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // 1. Configure the Virtual Device
    // We must tell the OS *exactly* what this device can do.
    // If we don't enable a button here, the OS will ignore it later.
    let mut keys = AttributeSet::<Key>::new();
    keys.insert(Key::BTN_LEFT);
    keys.insert(Key::BTN_RIGHT);
    keys.insert(Key::BTN_MIDDLE);
    keys.insert(Key::BTN_SIDE);      // Back button
    keys.insert(Key::BTN_EXTRA);     // Forward button

    let mut rels = AttributeSet::<RelativeAxisType>::new();
    rels.insert(RelativeAxisType::REL_X);
    rels.insert(RelativeAxisType::REL_Y);
    rels.insert(RelativeAxisType::REL_WHEEL); // Scroll wheel

    let mut device = VirtualDeviceBuilder::new()?
        .name("Rift Virtual Mouse")
        .with_keys(&keys)?
        .with_relative_axes(&rels)?
        .build()
        .expect("Failed to create virtual device. Do you have permission on /dev/uinput?");

    // FIX: Just print the name directly, don't ask the device object for it.
    println!("👻 Rift Receiver Active. Device created: Rift Virtual Mouse");

    // 2. Setup Network
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", args.port))?;
    println!("👂 Listening on UDP port {}", args.port);

    // 3. The Loop
    let mut buf = [0u8; 1024];
    loop {
        let (amt, _src) = socket.recv_from(&mut buf)?;
        
        if let Ok(packet) = bincode::deserialize::<InputPacket>(&buf[..amt]) {
            // Reconstruct the Event
            let event = InputEvent::new(
                EventType(packet.e_type),
                packet.code,
                packet.value
            );

            // Inject it into the Kernel
            device.emit(&[event])?;
        }
    }
}
