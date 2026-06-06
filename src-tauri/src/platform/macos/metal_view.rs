use metal::Device;

/// Verify Metal is available and return the default device.
pub fn default_device() -> Option<Device> {
    Device::system_default()
}
