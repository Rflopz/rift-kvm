# Rift-KVM: Software-Based KVM for Linux

Rift-KVM is a lightweight, software-defined KVM (Keyboard, Video, Mouse) solution designed for Linux systems. It uses the `evdev` and `uinput` kernel interfaces to capture physical input on one machine and replicate it as a virtual device on another over a network.

## Project Structure

- **`rift-core` (The Transmitter):** Hooks into physical input devices (e.g., `/dev/input/eventX`) using `evdev`, serializes input events using `bincode`, and transmits them via UDP.
- **`rift-receiver` (The Receiver):** Listens for incoming UDP packets, deserializes them, and injects the events into the kernel using a virtual `uinput` device.
- **`rift-ui` (The Interface):** (Placeholder) Intended to be the user interface for managing connections and configuration.

## Technologies

- **Language:** Rust (Edition 2024)
- **Input Handling:** `evdev` crate for reading physical devices and `uinput` for creating virtual ones.
- **Networking:** Standard library `UdpSocket`.
- **Serialization:** `serde` and `bincode`.
- **CLI Parsing:** `clap` (derive API).

## Building and Running

### Prerequisites

- **Permissions:** 
    - `rift-core` requires read access to `/dev/input/event*` and access to `/dev/uinput` for local cursor replication.
    - `rift-receiver` requires access to `/dev/uinput`.
    - You may need to add your user to the `input` and `uinput` groups or run with `sudo`.

### Build

```bash
cargo build --release
```

### Run Transmitter (rift-core)

Find your input device path (e.g., using `libinput list-devices` or checking `/dev/input/by-id/`).

```bash
# --width specifies the boundary where the mouse transitions to the remote screen
cargo run -p rift-core -- --device /dev/input/eventX --target <receiver-ip>:9000 --width 1920
```

### Run Receiver (rift-receiver)

```bash
cargo run -p rift-receiver -- --port 9000
```

## How Screen Crossing Works

1. **Grab:** `rift-core` takes exclusive control of your physical mouse (`grab`).
2. **Local Virtual Mouse:** It creates a virtual mouse on your main computer to move the local cursor.
3. **Boundary Detection:** It tracks the "virtual" X position of your mouse.
4. **Transition:** 
   - When `X < width`, it sends events to the **local** virtual mouse.
   - When `X >= width`, it sends events to the **remote** receiver via UDP.
   - Moving back from the remote screen to the left edge of the main screen will switch control back to Local mode.
