use botadapt_plugin_sdk::prelude::*;
use serde::Deserialize;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn dice(&mut self, sides: u32) -> u32 {
        (self.next() % sides as u64) as u32 + 1
    }
}

#[derive(Debug, Deserialize)]
struct DiceExpr {
    count: u32,
    sides: u32,
}

fn parse_dice(text: &str) -> Option<DiceExpr> {
    let text = text.trim();
    let (num, rest) = text.split_once('d')?;
    let count: u32 = num.parse().ok()?;
    let sides: u32 = rest.parse().ok()?;
    if count == 0 || count > 99 || sides == 0 || sides > 10000 {
        return None;
    }
    Some(DiceExpr { count, sides })
}

fn roll(expr: &DiceExpr, rng: &mut Rng) -> String {
    let mut results = Vec::with_capacity(expr.count as usize);
    let mut total: u32 = 0;
    for _ in 0..expr.count {
        let v = rng.dice(expr.sides);
        results.push(v);
        total += v;
    }

    if expr.count == 1 {
        format!("{}d{} = {}", expr.count, expr.sides, results[0])
    } else {
        let parts: Vec<String> = results.iter().map(|v| v.to_string()).collect();
        format!("{}d{} = {} = {}", expr.count, expr.sides, parts.join("+"), total)
    }
}

fn write_result(actions: &[Action]) -> i64 {
    let json = serde_json::to_vec(actions).unwrap_or_default();
    if json.is_empty() {
        return 0;
    }
    let ptr = json.as_ptr() as i32;
    let len = json.len() as i32;
    core::mem::forget(json);
    ((ptr as i64) << 32) | (len as i64 & 0x7FFFFFFF)
}

#[no_mangle]
pub extern "C" fn plugin_version() -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn plugin_handle_event(event_ptr: i32, event_len: i32) -> i64 {
    if event_ptr == 0 || event_len <= 0 {
        return 0;
    }

    let event_bytes = unsafe {
        core::slice::from_raw_parts(event_ptr as *const u8, event_len as usize)
    };

    let event: Event = match serde_json::from_slice(event_bytes) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    let msg_text = match &event.kind {
        EventKind::Message(msg) => &msg.content.text,
        _ => return 0,
    };

    let expr = match parse_dice(msg_text) {
        Some(e) => e,
        None => return 0,
    };

    let mut rng = Rng::new(event.timestamp as u64);
    let result_text = roll(&expr, &mut rng);

    let target = MessageTarget {
        platform: event.platform.clone(),
        user_id: match &event.kind {
            EventKind::Message(msg) => msg.user_id.clone(),
            _ => return 0,
        },
        group_id: match &event.kind {
            EventKind::Message(msg) => msg.group_id.clone(),
            _ => None,
        },
        channel_id: match &event.kind {
            EventKind::Message(msg) => msg.channel_id.clone(),
            _ => None,
        },
        adapter_instance: event.source_adapter.clone(),
    };

    let actions = vec![Action::SendMessage {
        target,
        content: MessageContent {
            text: result_text,
            mentions: vec![],
            attachments: vec![],
        },
    }];

    write_result(&actions)
}
