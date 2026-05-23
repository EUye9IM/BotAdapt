use botadapt_plugin_sdk::prelude::*;
use serde::Deserialize;

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

fn roll<RR: rand::Rng>(expr: &DiceExpr, rng: &mut RR) -> String {
    let mut results = Vec::with_capacity(expr.count as usize);
    let mut total: u32 = 0;
    for _ in 0..expr.count {
        let v = rng.gen_range(1..=expr.sides);
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

fn write_result(events: &[PluginEvent]) -> i64 {
    let json = serde_json::to_vec(events).unwrap_or_default();
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

    let event: AdapterEvent = match serde_json::from_slice(event_bytes) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    let msg = match &event {
        AdapterEvent::Message(msg) => msg,
    };

    let expr = match parse_dice(&msg.content.text) {
        Some(e) => e,
        None => return 0,
    };

    let mut rng = rand::thread_rng();
    let result_text = roll(&expr, &mut rng);

    let events = vec![PluginEvent::Message(MessageEvent {
        meta: msg.meta.clone(),
        content: MessageContent {
            text: result_text,
        },
    })];

    write_result(&events)
}
