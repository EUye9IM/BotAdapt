use std::path::Path;

use wasmtime::component::{bindgen, Component, Linker};
use wasmtime::{Engine, Store};

use crate::core::events::{BotEvent, MessageContent};
use crate::core::plugin::{Action, Plugin, PluginFactory};

bindgen!("tinybot-plugin" in "src/core/plugin.wit");

use tinybot::plugin::types as wit;

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
        let mut store = Store::new(&self.engine, ());
        let linker = Linker::new(&self.engine);
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
        let mut store = Store::new(&self.engine, ());
        let linker = Linker::new(&self.engine);
        let instance = TinybotPlugin::instantiate(&mut store, &self.component, &linker)?;
        Ok(Box::new(WasmPlugin { store, instance }))
    }
}

pub struct WasmPlugin {
    store: Store<()>,
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
