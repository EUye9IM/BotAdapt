use crate::core::{config, events::BotEvent};
pub struct SessionMgr {}
pub struct Session {}

impl SessionMgr {
    pub fn new(cfg: config::Config) -> Self {
        return SessionMgr {};
    }
    pub fn get_session(&self, bid: &str, evt: BotEvent) -> Option<Session> {
        todo!()
    }
}
