use std::collections::HashMap;

use crate::core::events::{self, BotEvent};

/// 插件工厂：全局持有，负责创建插件实例
pub trait PluginFactory: Send + Sync {
    fn name(&self) -> &str;
    fn active(&self, evt: &BotEvent) -> bool;
    fn create(&self) -> anyhow::Result<Box<dyn Plugin>>;
}

/// 插件实例：每个 Session 持有一个，随 Session 生命周期
pub trait Plugin: Send {
    fn handle(&mut self, evt: &BotEvent) -> anyhow::Result<Action>;
}

/// 插件返回的操作指令
pub struct Action {
    pub finish: bool,
    pub reply: events::MessageContent,
}

/// 插件管理器：全局持有所有插件工厂
pub struct PluginManager {
    factories: HashMap<String, Box<dyn PluginFactory>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// 注册插件工厂
    pub fn register(&mut self, factory: Box<dyn PluginFactory>) {
        self.factories.insert(factory.name().to_owned(), factory);
    }

    /// 返回所有已注册插件名称
    pub fn names(&self) -> Vec<&str> {
        self.factories.keys().map(|s| s.as_str()).collect()
    }

    /// 获取插件实例，不存在或创建失败返回 None
    pub fn get(&self, name: &str) -> Option<Box<dyn Plugin>> {
        match self.factories.get(name)?.create() {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::warn!("创建插件实例 {} 失败: {}", name, e);
                None
            }
        }
    }

    /// 检查指定插件对此事件是否激活
    pub fn is_active(&self, name: &str, evt: &BotEvent) -> bool {
        self.factories
            .get(name)
            .map(|f| f.active(evt))
            .unwrap_or(false)
    }
}
