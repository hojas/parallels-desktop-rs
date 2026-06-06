use crate::vm::config::VmConfig;

/// Expand ~ in a path string.
fn expand_tilde(path: &str) -> String {
    if path.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            return home.to_string_lossy().to_string() + &path[1..];
        }
    }
    path.to_string()
}

/// Build QEMU command-line arguments from a VmConfig.
pub fn build_qemu_args(config: &VmConfig) -> Vec<String> {
    let disk = expand_tilde(&config.disk_path.to_string_lossy());
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
        format!("if=virtio,format=qcow2,file={disk}"),
    ];

    if let Some(ref iso) = config.iso_path {
        let iso_expanded = expand_tilde(&iso.to_string_lossy());
        args.push("-drive".into());
        args.push(format!(
            "if=virtio,format=raw,file={iso_expanded},media=cdrom"
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
