use crate::vm::config::VmConfig;

/// Build QEMU command-line arguments from a VmConfig.
pub fn build_qemu_args(config: &VmConfig) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "qemu-system-aarch64".into(),
        "-M".into(),
        "virt,accel=hvf,highmem=on".into(),
        "-cpu".into(),
        "host".into(),
        "-smp".into(),
        config.cpu_count.to_string(),
        "-m".into(),
        config.memory_mb.to_string(),
        "-serial".into(),
        "stdio".into(),
        "-qmp".into(),
        format!(
            "unix:{},server=on,wait=off",
            config.qmp_socket_path().display()
        ),
        "-monitor".into(),
        "none".into(),
        "-drive".into(),
        format!(
            "if=virtio,format=qcow2,file={}",
            config.disk_path.display()
        ),
    ];

    if let Some(ref iso) = config.iso_path {
        args.push("-drive".into());
        args.push(format!(
            "if=virtio,format=raw,file={},media=cdrom",
            iso.display()
        ));
    }

    args.extend_from_slice(&[
        "-nic".into(),
        "user,model=virtio-net-pci".into(),
        "-device".into(),
        "qemu-xhci".into(),
        "-device".into(),
        "usb-kbd".into(),
        "-device".into(),
        "usb-tablet".into(),
        "-device".into(),
        "ramfb".into(),
    ]);

    // Shared folders via VirtFS (9p)
    for folder in &config.shared_folders {
        args.push("-virtfs".into());
        args.push(format!(
            "local,path={folder},mount_tag=hostshare,security_model=mapped-xattr"
        ));
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::config::VmConfig;

    #[test]
    fn includes_hvf_acceleration() {
        let config = VmConfig::new("test", "/tmp/test.qcow2");
        let args = build_qemu_args(&config);
        let joined = args.join(" ");
        assert!(joined.contains("accel=hvf"));
    }

    #[test]
    fn includes_iso_when_present() {
        let mut config = VmConfig::new("test", "/tmp/test.qcow2");
        config.iso_path = Some("/tmp/debian.iso".into());
        let args = build_qemu_args(&config);
        assert!(args.join(" ").contains("media=cdrom"));
    }

    #[test]
    fn includes_ramfb() {
        let config = VmConfig::new("test", "/tmp/test.qcow2");
        let args = build_qemu_args(&config);
        assert!(args.join(" ").contains("ramfb"));
    }
}
