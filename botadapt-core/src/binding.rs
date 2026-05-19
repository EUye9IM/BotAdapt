use std::collections::HashMap;

/// 按 adapter 实例分组的 channel 绑定表。
///
/// channel_id 格式: `"{type}:{id}"` (如 `"group:123456"`, `"c2c:*"`)
/// 支持通配符 `*`：精确匹配优先，其次尝试 `"{type}:*"` 通配，最后全通配 `*`。
#[derive(Debug, Default)]
pub struct ChannelBinding {
    bindings: HashMap<String, HashMap<String, Vec<String>>>,
}

impl ChannelBinding {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn add(&mut self, instance_id: &str, channel_id: String, plugins: Vec<String>) {
        self.bindings
            .entry(instance_id.to_string())
            .or_default()
            .insert(channel_id, plugins);
    }

    pub fn resolve(&self, source_adapter: &str, channel_id: &str) -> Vec<String> {
        if let Some(instance_bindings) = self.bindings.get(source_adapter) {
            // 1. 精确匹配
            if let Some(plugins) = instance_bindings.get(channel_id) {
                if !plugins.is_empty() {
                    return plugins.clone();
                }
            }

            // 2. 通配匹配："{type}:*"
            let parts: Vec<&str> = channel_id.splitn(2, ':').collect();
            if parts.len() == 2 {
                let wild = format!("{}:*", parts[0]);
                if let Some(plugins) = instance_bindings.get(&wild) {
                    if !plugins.is_empty() {
                        return plugins.clone();
                    }
                }
            }

            // 3. 全通配
            if let Some(plugins) = instance_bindings.get("*") {
                return plugins.clone();
            }
        }

        Vec::new()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}
