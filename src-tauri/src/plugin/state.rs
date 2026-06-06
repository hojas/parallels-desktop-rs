use crate::vm::manager::VmManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub vms: Arc<Mutex<HashMap<String, VmManager>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            vms: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register(&self, manager: VmManager) {
        let id = manager.config.id.clone();
        self.vms.lock().await.insert(id, manager);
    }

    pub async fn get(&self, id: &str) -> Option<VmManager> {
        self.vms.lock().await.remove(id)
    }

    pub async fn put(&self, id: String, manager: VmManager) {
        self.vms.lock().await.insert(id, manager);
    }
}
