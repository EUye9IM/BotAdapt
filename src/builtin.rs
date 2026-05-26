fn builtin_commands() -> Vec<BuiltinCommand> {
    vec![BuiltinCommand {
        name: "ping",
        description: "回复 pong",
        handler: Box::new(|event, _ctx| {
            tracing::debug!("receive {:?}", event);
            let AdapterEvent::Message(m) = event;
            let MessageMeta::Private(p) = m.meta.clone();
            Ok(vec![PluginEvent::Message(MessageEvent {
                meta: MessageMeta::Private(PrivateMeta { user_id: p.user_id }),
                content: MessageContent {
                    text: "pong!".to_owned(),
                },
            })])
        }),
    }]
}
