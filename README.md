# Parallels Desktop RS

A Parallels Desktop-like virtual machine manager for Apple Silicon Macs, built with **Tauri v2 + Rust**. Runs ARM64 Linux and Windows 11 VMs using QEMU with Hypervisor.framework acceleration.

## Features

- **VM lifecycle** — create, start, stop, delete VMs
- **Serial console** — real-time guest output in the UI
- **QMP control** — graceful shutdown, status queries, input via QEMU Machine Protocol
- **Snapshots** — save/restore/delete VM snapshots
- **Shared folders** — VirtFS (9p) host-guest file sharing
- **Suspend/resume** — save/restore full VM state
- **ramfb display** — framebuffer with Metal rendering pipeline
- **Windows 11 ARM** — EDK2 UEFI + swtpm TPM 2.0 config
- **Dark UI** — React + TypeScript frontend

## Prerequisites

- Apple Silicon Mac (M1/M2/M3/M4) running macOS 14+
- [Rust](https://rustup.rs) 1.77+
- [Node.js](https://nodejs.org) 20+
- [QEMU](https://www.qemu.org) (`brew install qemu`)

## Quick Start

```bash
brew install qemu
git clone https://github.com/hojas/parallels-desktop-rs.git
cd parallels-desktop-rs
npm install
cargo tauri dev
```

## Usage

### 1. Create a disk image

```bash
qemu-img create -f qcow2 ~/VMs/debian.qcow2 20G
```

### 2. Download a guest OS

- [Debian ARM64](https://cdimage.debian.org/debian-cd/current/arm64/iso-cd/)
- [Ubuntu Server ARM64](https://ubuntu.com/download/server/arm)
- [Windows 11 ARM](https://www.microsoft.com/software-download/windows11arm64)

### 3. Launch the app

1. Open the app (`cargo tauri dev` or `.app` bundle)
2. Fill in disk path + ISO path in the sidebar
3. Click **Save VM** → **Start VM**
4. Watch boot output in the serial console
5. Click **Stop VM** to shut down

### Windows 11 ARM

Requires extra setup:

```bash
brew install swtpm
cp /opt/homebrew/share/qemu/edk2-aarch64-code.fd AAVMF_CODE.fd
cp /opt/homebrew/share/qemu/edk2-aarch64-vars.fd AAVMF_VARS.fd
```

## Architecture

```
Tauri Window (macOS)
├── WKWebView (React UI: VM list, terminal, snapshots)
├── MTKView (Metal-rendered VM display)
└── Rust Backend
    ├── VmManager — state machine + process lifecycle
    ├── QmpClient — dual-channel JSON/Unix socket
    ├── SnapshotManager — savevm/loadvm/delvm
    └── QEMU subprocess (aarch64-softmmu + HVF)
```

## Project Structure

```
src-tauri/src/
├── lib.rs, main.rs
├── vm/{config, manager, snapshot}.rs
├── vm/qemu/{command_builder, qmp_client, tpm, windows11}.rs
├── platform/macos/{metal_view, input}.rs
└── plugin/{commands, state, events}.rs
src/
├── App.tsx, App.css, index.css
```

## Tauri Commands

| Command | Description |
|---------|-------------|
| `create_vm` | Save VM configuration |
| `list_vms` | List saved VMs |
| `delete_vm` | Remove VM and data |
| `start_vm` | Boot VM (spawns QEMU) |
| `stop_vm` | Graceful shutdown |
| `snapshot_*` | Create/list/restore/delete snapshots |
| `suspend_vm` | Save VM state to file |
| `resume_vm` | Restore from saved state |

## Development

```bash
cargo test --lib          # Unit tests
npm run build             # TypeScript check
cargo tauri build --debug # Debug .app + DMG
cargo clippy -- -D warnings
```

## License

MIT
