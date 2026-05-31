use std::path::Path;

use wasmtime::component::{bindgen, Component, Linker, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use crate::core::events::{BotEvent, MessageContent};
use crate::core::plugin::{Action, Plugin, PluginFactory};

bindgen!("tinybot-plugin" in "wit/plugin.wit");

use tinybot::plugin::types as wit;

struct WasmState {
    wasi_ctx: WasiCtx,
    resource_table: ResourceTable,
}

impl WasiView for WasmState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.resource_table,
        }
    }
}

impl WasmState {
    fn new(_engine: &Engine) -> Self {
        Self {
            wasi_ctx: WasiCtx::builder().build(),
            resource_table: ResourceTable::new(),
        }
    }
}

pub struct WasmPluginFactory {
    engine: Engine,
    component: Component,
}

impl WasmPluginFactory {
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let engine = Engine::default();
        let component = Component::from_file(&engine, path)?;
        Ok(Self { engine, component })
    }

    fn linker(&self) -> anyhow::Result<Linker<WasmState>> {
        let mut linker = Linker::new(&self.engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        Ok(linker)
    }

    fn convert_event(evt: &BotEvent) -> wit::BotEvent {
        match evt {
            BotEvent::Message(msg) => wit::BotEvent::Message(wit::Message {
                target_type: msg.target_type.clone(),
                target: msg.target.clone(),
                content: wit::MessageContent {
                    text: msg.content.text.clone(),
                },
            }),
        }
    }

    fn convert_action(a: wit::Action) -> Action {
        Action {
            finish: a.finish,
            reply: a.reply.map(|c| MessageContent { text: c.text }),
        }
    }
}

impl PluginFactory for WasmPluginFactory {
    fn active(&self, evt: &BotEvent) -> bool {
        let mut store = Store::new(&self.engine, WasmState::new(&self.engine));
        let linker = match self.linker() {
            Ok(l) => l,
            Err(_) => return false,
        };
        let Ok(instance) =
            TinybotPlugin::instantiate(&mut store, &self.component, &linker)
        else {
            return false;
        };
        let event = Self::convert_event(evt);
        instance
            .interface0
            .call_active(&mut store, &event)
            .unwrap_or(false)
    }

    fn create(&self) -> anyhow::Result<Box<dyn Plugin>> {
        let mut store = Store::new(&self.engine, WasmState::new(&self.engine));
        let linker = self.linker()?;
        let instance = TinybotPlugin::instantiate(&mut store, &self.component, &linker)?;
        Ok(Box::new(WasmPlugin { store, instance }))
    }
}

pub struct WasmPlugin {
    store: Store<WasmState>,
    instance: TinybotPlugin,
}

impl Plugin for WasmPlugin {
    fn handle(&mut self, evt: &BotEvent) -> anyhow::Result<Action> {
        let event = WasmPluginFactory::convert_event(evt);
        let action = self
            .instance
            .interface0
            .call_handle(&mut self.store, &event)?;
        Ok(WasmPluginFactory::convert_action(action))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::{BotEvent, Message, MessageContent};

    #[test]
    fn convert_event_message() {
        let evt = BotEvent::Message(Message {
            target_type: "group".into(),
            target: "123".into(),
            content: MessageContent { text: "hi".into() },
        });
        let wit_evt = WasmPluginFactory::convert_event(&evt);
        match wit_evt {
            wit::BotEvent::Message(ref m) => {
                assert_eq!(m.target_type, "group");
                assert_eq!(m.target, "123");
                assert_eq!(m.content.text, "hi");
            }
        }
    }

    #[test]
    fn convert_action_full() {
        let wit_action = wit::Action {
            finish: false,
            reply: Some(wit::MessageContent {
                text: "reply text".into(),
            }),
        };
        let action = WasmPluginFactory::convert_action(wit_action);
        assert!(!action.finish);
        assert_eq!(action.reply.unwrap().text, "reply text");
    }

    #[test]
    fn convert_action_none_reply() {
        let wit_action = wit::Action {
            finish: true,
            reply: None,
        };
        let action = WasmPluginFactory::convert_action(wit_action);
        assert!(action.finish);
        assert!(action.reply.is_none());
    }

    #[test]
    fn convert_event_roundtrip_preserves_text() {
        let original = MessageContent {
            text: "hello 世界".into(),
        };
        let evt = BotEvent::Message(Message {
            target_type: "channel".into(),
            target: "abc".into(),
            content: original.clone(),
        });
        let wit_evt = WasmPluginFactory::convert_event(&evt);
        match wit_evt {
            wit::BotEvent::Message(ref m) => {
                assert_eq!(m.content.text, original.text);
                assert_eq!(m.target_type, "channel");
                assert_eq!(m.target, "abc");
            }
        }
    }

    #[test]
    fn integration_load_and_active() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/plugins/hello/target/wasm32-wasip2/debug/hello.wasm"
        );
        let factory = WasmPluginFactory::from_file(path).expect("load hello.wasm");

        let evt = BotEvent::Message(Message {
            target_type: "group".into(),
            target: "123".into(),
            content: MessageContent {
                text: "hello".into(),
            },
        });
        assert!(factory.active(&evt));

        let evt_other = BotEvent::Message(Message {
            target_type: "group".into(),
            target: "123".into(),
            content: MessageContent {
                text: "nope".into(),
            },
        });
        assert!(!factory.active(&evt_other));
    }

    #[test]
    fn integration_create_and_handle() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/plugins/hello/target/wasm32-wasip2/debug/hello.wasm"
        );
        let factory = WasmPluginFactory::from_file(path).expect("load hello.wasm");
        let mut plugin = factory.create().expect("create plugin instance");

        let evt = BotEvent::Message(Message {
            target_type: "group".into(),
            target: "123".into(),
            content: MessageContent {
                text: "world".into(),
            },
        });
        let action = plugin.handle(&evt).expect("handle event");
        assert!(action.finish);
        assert_eq!(action.reply.unwrap().text, "hello world");
    }
}
