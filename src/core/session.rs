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
