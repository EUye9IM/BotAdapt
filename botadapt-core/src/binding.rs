use std::collections::HashMap;

/// channel_id → 插件名列表 的绑定表。
///
/// channel_id 格式: `"{platform}:{type}:{id}"`
/// 支持通配符 `*`：精确匹配优先，其次尝试通配。
#[derive(Debug, Default)]
pub struct ChannelBinding {
    bindings: HashMap<String, Vec<String>>,
}

impl ChannelBinding {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn add(&mut self, channel_id: String, plugins: Vec<String>) {
        self.bindings.insert(channel_id, plugins);
    }

    pub fn resolve(&self, channel_id: &str) -> Vec<String> {
        // 1. 精确匹配
        if let Some(plugins) = self.bindings.get(channel_id) {
            if !plugins.is_empty() {
                return plugins.clone();
            }
        }

        // 2. 通配匹配：逐个尝试 `*` 规则
        //    将 channel_id 拆分为 [platform, type, id]，逐步替换尾部为 `*`
        let parts: Vec<&str> = channel_id.splitn(3, ':').collect();
        if parts.len() == 3 {
            // 尝试 "{platform}:{type}:*"
            let wild = format!("{}:{}:*", parts[0], parts[1]);
            if let Some(plugins) = self.bindings.get(&wild) {
                if !plugins.is_empty() {
                    return plugins.clone();
                }
            }
        }

        // 3. 全通配
        if let Some(plugins) = self.bindings.get("*") {
            return plugins.clone();
        }

        Vec::new()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}
