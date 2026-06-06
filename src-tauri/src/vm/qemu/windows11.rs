use crate::vm::config::VmConfig;

/// Build QEMU arguments for Windows 11 ARM on Apple Silicon.
///
/// Requires:
/// - EDK2 AArch64 UEFI (AAVMF_CODE.fd / AAVMF_VARS.fd)
/// - TPM 2.0 via swtpm
/// - VirtIO drivers ISO for storage/network during install
/// - ramfb or virtio-gpu for display
pub fn build_windows11_args(config: &VmConfig, tpm_socket: &str) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "qemu-system-aarch64".into(),
        "-M".into(), "virt,accel=hvf,highmem=on,gic-version=max".into(),
        "-cpu".into(), "host".into(),
        "-smp".into(), config.cpu_count.to_string(),
        "-m".into(), config.memory_mb.to_string(),
        // UEFI firmware
        "-drive".into(), "if=pflash,format=raw,file=AAVMF_CODE.fd,readonly=on".into(),
        "-drive".into(), "if=pflash,format=raw,file=AAVMF_VARS.fd".into(),
        // Primary disk
        "-drive".into(), format!("if=virtio,format=qcow2,file={}", config.disk_path.display()),
    ];

    if let Some(ref iso) = config.iso_path {
        args.push("-drive".into());
        args.push(format!("if=virtio,format=raw,file={},media=cdrom", iso.display()));
    }

    // VirtIO drivers ISO
    args.push("-drive".into());
    args.push("if=virtio,format=raw,file=virtio-win.iso,media=cdrom".into());

    // TPM 2.0
    args.append(&mut crate::vm::qemu::tpm::tpm_qemu_args(tpm_socket));

    // Networking
    args.push("-nic".into()); args.push("user,model=virtio-net-pci".into());

    // USB
    args.push("-device".into()); args.push("qemu-xhci".into());
    args.push("-device".into()); args.push("usb-kbd".into());
    args.push("-device".into()); args.push("usb-tablet".into());

    // Display
    args.push("-device".into()); args.push("ramfb".into());

    // QMP
    args.push("-qmp".into());
    args.push(format!("unix:{},server=on,wait=off", config.qmp_socket_path().display()));

    args.push("-monitor".into()); args.push("none".into());
    args.push("-serial".into()); args.push("stdio".into());

    args
}
