use std::collections::HashMap;

use crate::core::plugin::Plugin;

pub struct Session {
    pub bid: String,
    pub target_type: String,
    pub target_id: String,
    pub plugin: Box<dyn Plugin>,
}

pub struct SessionMgr {
    sessions: HashMap<(String, String, String), Session>,
}

impl SessionMgr {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn get_session(
        &mut self,
        bid: &str,
        ttype: &str,
        tid: &str,
    ) -> Option<&mut Session> {
        self.sessions
            .get_mut(&(bid.to_owned(), ttype.to_owned(), tid.to_owned()))
    }

    pub fn create_session(
        &mut self,
        bid: &str,
        ttype: &str,
        tid: &str,
        plugin: Box<dyn Plugin>,
    ) {
        let key = (bid.to_owned(), ttype.to_owned(), tid.to_owned());
        self.sessions.insert(
            key,
            Session {
                bid: bid.to_owned(),
                target_type: ttype.to_owned(),
                target_id: tid.to_owned(),
                plugin,
            },
        );
    }

    pub fn remove_session(&mut self, bid: &str, ttype: &str, tid: &str) {
        self.sessions
            .remove(&(bid.to_owned(), ttype.to_owned(), tid.to_owned()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::BotEvent;
    use crate::core::plugin::{Action, Plugin};

    struct DummyPlugin;
    impl Plugin for DummyPlugin {
        fn handle(&mut self, _evt: &BotEvent) -> anyhow::Result<Action> {
            Ok(Action {
                finish: true,
                reply: None,
            })
        }
    }

    fn dummy_plugin() -> Box<dyn Plugin> {
        Box::new(DummyPlugin)
    }

    #[test]
    fn new_session_mgr_is_empty() {
        let mgr = SessionMgr::new();
        assert!(mgr.sessions.is_empty());
    }

    #[test]
    fn create_and_get_session() {
        let mut mgr = SessionMgr::new();
        mgr.create_session("bot1", "group", "123", dummy_plugin());
        assert!(mgr.get_session("bot1", "group", "123").is_some());
    }

    #[test]
    fn get_nonexistent_session() {
        let mut mgr = SessionMgr::new();
        assert!(mgr.get_session("bot1", "group", "123").is_none());
    }

    #[test]
    fn remove_session() {
        let mut mgr = SessionMgr::new();
        mgr.create_session("bot1", "group", "123", dummy_plugin());
        mgr.remove_session("bot1", "group", "123");
        assert!(mgr.get_session("bot1", "group", "123").is_none());
    }

    #[test]
    fn different_keys_independent() {
        let mut mgr = SessionMgr::new();
        mgr.create_session("bot1", "group", "123", dummy_plugin());
        mgr.create_session("bot2", "channel", "abc", dummy_plugin());
        assert!(mgr.get_session("bot1", "group", "123").is_some());
        assert!(mgr.get_session("bot2", "channel", "abc").is_some());
        mgr.remove_session("bot1", "group", "123");
        assert!(mgr.get_session("bot1", "group", "123").is_none());
        assert!(mgr.get_session("bot2", "channel", "abc").is_some());
    }

    #[test]
    fn overwrite_existing_session() {
        let mut mgr = SessionMgr::new();
        mgr.create_session("bot1", "group", "123", dummy_plugin());
        mgr.create_session("bot1", "group", "123", dummy_plugin());
        let count = mgr.sessions.len();
        assert_eq!(count, 1);
    }
}
