use clap::Parser;
use evdev::uinput::VirtualDeviceBuilder;
use evdev::{AttributeSet, Device, EventType, Key, RelativeAxisType};
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

#[derive(Serialize, Deserialize, Debug)]
enum Packet {
    Event { e_type: u16, code: u16, value: i32 },
    ConfigRequest,
    ConfigResponse { width: i32 },
}

#[derive(PartialEq)]
enum Mode {
    Local,
    Remote,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // 0. Detect Screen Width (using display-info)
    let screen_width = display_info::DisplayInfo::all()
        .map(|displays| {
            displays.iter()
                .find(|d| d.is_primary)
                .map(|d| d.width as i32)
                .unwrap_or(1920)
        })
        .unwrap_or(1920);

    println!("🖥️  Local Screen Width: {}", screen_width);

    // 1. Setup Network and Handshake
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(std::time::Duration::from_millis(1000)))?;
    
    println!("🤝 Discovering Remote Screen Width...");
    let config_req = bincode::serialize(&Packet::ConfigRequest).unwrap();
    let _ = socket.send_to(&config_req, &args.target);
    
    let mut buf = [0u8; 1024];
    let mut remote_width = 1920; // Default fallback
    if let Ok((amt, _)) = socket.recv_from(&mut buf) {
        if let Ok(Packet::ConfigResponse { width }) = bincode::deserialize::<Packet>(&buf[..amt]) {
            remote_width = width;
            println!("✅ Discovered Remote Width: {}", remote_width);
        }
    } else {
        println!("⚠️  Remote not responding. Using default width: {}", remote_width);
    }
    
    socket.set_read_timeout(None).expect("Could not clear timeout");

    // 2. Setup Local Virtual Device
    let mut keys = AttributeSet::<Key>::new();
    for i in 0..512 {
        keys.insert(Key::new(i));
    }

    let mut rels = AttributeSet::<RelativeAxisType>::new();
    rels.insert(RelativeAxisType::REL_X);
    rels.insert(RelativeAxisType::REL_Y);
    rels.insert(RelativeAxisType::REL_WHEEL);
    rels.insert(RelativeAxisType::REL_HWHEEL);

    let mut local_virtual = VirtualDeviceBuilder::new()?
        .name("Rift Local Virtual Mouse")
        .with_keys(&keys)?
        .with_relative_axes(&rels)?
        .build()
        .expect("Failed to create local virtual device. Try running with sudo.");

    // 3. Open and GRAB the physical device
    let mut physical_device = Device::open(&args.device)?;
    if let Err(e) = physical_device.grab() {
        eprintln!("⚠️  WARNING: Could not grab device: {}. Use sudo.", e);
    } else {
        println!("✅ Device grabbed successfully.");
    }

    let mut mode = Mode::Local;
    let mut virtual_x = screen_width / 2; 

    loop {
        let mut events = Vec::new();
        for event in physical_device.fetch_events()? {
            events.push(event);
        }

        for event in events {
            if event.event_type() == EventType::RELATIVE && event.code() == RelativeAxisType::REL_X.0 {
                virtual_x += event.value();
                
                // Virtual Coordinate Space: [-remote_width, screen_width]
                if virtual_x < -remote_width {
                    virtual_x = -remote_width;
                } else if virtual_x > screen_width {
                    virtual_x = screen_width;
                }

                // Transition Logic
                match mode {
                    Mode::Local if virtual_x < 0 => {
                        mode = Mode::Remote;
                        println!("🚀 Switched to REMOTE (Left)");
                    }
                    Mode::Remote if virtual_x >= 0 => {
                        mode = Mode::Local;
                        println!("🏠 Switched to LOCAL");
                    }
                    _ => {}
                }
            }

            // Route events based on current mode
            match mode {
                Mode::Local => {
                    local_virtual.emit(&[event])?;
                }
                Mode::Remote => {
                    let packet = Packet::Event {
                        e_type: event.event_type().0,
                        code: event.code(),
                        value: event.value(),
                    };
                    let encoded = bincode::serialize(&packet).unwrap();
                    let _ = socket.send_to(&encoded, &args.target);
                }
            }
        }
    }
}
