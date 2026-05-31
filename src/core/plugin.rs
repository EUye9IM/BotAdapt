use std::collections::HashMap;

use crate::core::events::{self, BotEvent};

/// 插件工厂：全局持有，负责创建插件实例
pub trait PluginFactory: Send + Sync {
    fn active(&self, evt: &BotEvent) -> bool;
    fn create(&self) -> anyhow::Result<Box<dyn Plugin>>;
}

/// 插件实例：每个 Session 持有一个，随 Session 生命周期
pub trait Plugin: Send {
    fn handle(&mut self, evt: &BotEvent) -> anyhow::Result<Action>;
}

/// 插件返回的操作指令
#[derive(Debug)]
pub struct Action {
    pub finish: bool,
    pub reply: Option<events::MessageContent>,
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
    pub fn register(&mut self, name: &str, factory: Box<dyn PluginFactory>) {
        self.factories.insert(name.to_owned(), factory);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::{Message, MessageContent};

    struct MockFactory {
        active_val: bool,
    }

    impl PluginFactory for MockFactory {
        fn active(&self, _evt: &BotEvent) -> bool {
            self.active_val
        }

        fn create(&self) -> anyhow::Result<Box<dyn Plugin>> {
            struct MockPlugin;
            impl Plugin for MockPlugin {
                fn handle(&mut self, _evt: &BotEvent) -> anyhow::Result<Action> {
                    Ok(Action {
                        finish: false,
                        reply: Some(MessageContent {
                            text: "mock".into(),
                        }),
                    })
                }
            }
            Ok(Box::new(MockPlugin))
        }
    }

    fn evt() -> BotEvent {
        BotEvent::Message(Message {
            target_type: "group".into(),
            target: "123".into(),
            content: MessageContent {
                text: "hi".into(),
            },
        })
    }

    #[test]
    fn register_and_names() {
        let mut mgr = PluginManager::new();
        mgr.register("test", Box::new(MockFactory { active_val: true }));
        let names = mgr.names();
        assert_eq!(names, vec!["test"]);
    }

    #[test]
    fn is_active_true() {
        let mut mgr = PluginManager::new();
        mgr.register("test", Box::new(MockFactory { active_val: true }));
        assert!(mgr.is_active("test", &evt()));
    }

    #[test]
    fn is_active_false() {
        let mut mgr = PluginManager::new();
        mgr.register("test", Box::new(MockFactory { active_val: false }));
        assert!(!mgr.is_active("test", &evt()));
    }

    #[test]
    fn is_active_missing_returns_false() {
        let mgr = PluginManager::new();
        assert!(!mgr.is_active("nope", &evt()));
    }

    #[test]
    fn get_returns_instance() {
        let mut mgr = PluginManager::new();
        mgr.register("test", Box::new(MockFactory { active_val: true }));
        assert!(mgr.get("test").is_some());
    }

    #[test]
    fn get_missing_returns_none() {
        let mgr = PluginManager::new();
        assert!(mgr.get("nope").is_none());
    }
}
