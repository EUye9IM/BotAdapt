use std::sync::Arc;

use anyhow::Result;

use crate::core::{adapter::Adapter, config::AdapterConfig};

pub fn new_adapter_from_config(cfg: &AdapterConfig) -> Result<Arc<dyn Adapter>> {
    todo!()
}
