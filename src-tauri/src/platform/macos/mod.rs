use anyhow::Context;

pub mod input;
pub mod metal_view;

/// Check that Hypervisor.framework is available.
pub fn check_hv_support() -> anyhow::Result<()> {
    let output = std::process::Command::new("sysctl")
        .args(["-n", "kern.hv_support"])
        .output()
        .context("failed to check kern.hv_support")?;

    if String::from_utf8_lossy(&output.stdout).trim() != "1" {
        anyhow::bail!("Hypervisor.framework is not available on this machine");
    }
    Ok(())
}

/// Verify we're on Apple Silicon.
pub fn check_apple_silicon() -> anyhow::Result<()> {
    if std::env::consts::ARCH != "aarch64" {
        anyhow::bail!("This application requires Apple Silicon (aarch64)");
    }
    Ok(())
}
