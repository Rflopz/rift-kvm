use clap::Parser;
use evdev::Device; // Removed unused InputEventKind
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about = "Rift Core: The Input Hijacker")]
struct Args {
    /// Path to the input device (e.g., /dev/input/eventX)
    #[arg(short, long)]
    device: PathBuf,

    /// Target address (e.g., 127.0.0.1:9000)
    #[arg(short, long, default_value = "127.0.0.1:9000")]
    target: String,
}

// The "Wire Format"
#[derive(Serialize, Deserialize, Debug)]
struct InputPacket {
    e_type: u16, 
    code: u16,   
    value: i32,  
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    println!("📡 Transmitter active. Target: {}", args.target);

    let mut device = Device::open(&args.device)?;
    println!("✅ Hooked into: {}", device.name().unwrap_or("Unknown"));

    loop {
        for event in device.fetch_events()? {
            // FIX: Use .event_type().0 to get the raw u16
            let packet = InputPacket {
                e_type: event.event_type().0, 
                code: event.code(),
                value: event.value(),
            };

            let encoded = bincode::serialize(&packet).unwrap();
            socket.send_to(&encoded, &args.target)?;
        }
    }
}
