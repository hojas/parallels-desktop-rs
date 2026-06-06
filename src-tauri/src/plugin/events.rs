use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VmStatusEvent {
    pub vm_id: String,
    pub state: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerialOutputEvent {
    pub vm_id: String,
    pub line: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VmErrorEvent {
    pub vm_id: String,
    pub message: String,
}

impl VmStatusEvent {
    pub fn new(vm_id: impl Into<String>, state: impl Into<String>) -> Self {
        Self {
            vm_id: vm_id.into(),
            state: state.into(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

impl SerialOutputEvent {
    pub fn new(vm_id: impl Into<String>, line: impl Into<String>) -> Self {
        Self { vm_id: vm_id.into(), line: line.into() }
    }
}

impl VmErrorEvent {
    pub fn new(vm_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self { vm_id: vm_id.into(), message: message.into() }
    }
}
