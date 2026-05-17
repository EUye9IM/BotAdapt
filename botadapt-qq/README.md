# botadapt-qq

QQ 官方机器人 API adapter 实现。基于 [QQ Bot API v2](https://bot.q.qq.com/wiki/develop/api-v2/)。

## 外部参考链接

- 官方文档首页: <https://bot.q.qq.com/wiki/develop/api-v2/>
- 接口调用与鉴权: <https://bot.q.qq.com/wiki/develop/api-v2/dev-prepare/interface-framework/api-use.html>
- 事件订阅与通知: <https://bot.q.qq.com/wiki/develop/api-v2/dev-prepare/interface-framework/event-emit.html>
- WebSocket 接入参考: <https://bot.q.qq.com/wiki/develop/api-v2/dev-prepare/interface-framework/reference.html>
- OpCode 列表: <https://bot.q.qq.com/wiki/develop/api-v2/dev-prepare/interface-framework/opcode.html>
- 获取 WSS 接入点: <https://bot.q.qq.com/wiki/develop/api-v2/openapi/wss/url_get.html>
- 获取分片 WSS 接入点: <https://bot.q.qq.com/wiki/develop/api-v2/openapi/wss/shard_url_get.html>
- 发送消息: <https://bot.q.qq.com/wiki/develop/api-v2/server-inter/message/send-receive/send.html>
- 消息事件: <https://bot.q.qq.com/wiki/develop/api-v2/server-inter/message/send-receive/event.html>
- 消息对象模型: <https://bot.q.qq.com/wiki/develop/api-v2/server-inter/message/template/model.html>
- 错误码: <https://bot.q.qq.com/wiki/develop/api-v2/openapi/error/error.html>
- SDK 参考 (Go): <https://github.com/tencent-connect/botgo>
- SDK 参考 (Python): <https://github.com/tencent-connect/botpy>
- SDK 参考 (Node): <https://github.com/tencent-connect/bot-node-sdk>

## API 交互规范

### 鉴权

1. POST `https://bots.qq.com/app/getAppAccessToken` — body: `{"appId": "...", "clientSecret": "..."}`
2. 返回 `{"access_token": "...", "expires_in": 7200}` (有效期 7200s)
3. 过期前 60s 内刷新获取新 token，旧 token 在 60s 内仍有效
4. 所有 HTTP API 请求头: `Authorization: QQBot {access_token}`

### WebSocket Payload 结构

```json
{
  "op": 0,
  "d": {},
  "s": 42,
  "t": "EVENT_NAME"
}
```

### OpCode 速查

| op  | 名称            | 方向    | 说明                   |
|-----|-----------------|---------|------------------------|
| 0   | Dispatch        | Receive | 服务端推送事件         |
| 1   | Heartbeat       | Send    | 客户端发心跳           |
| 2   | Identify        | Send    | 客户端鉴权             |
| 6   | Resume          | Send    | 恢复连接               |
| 7   | Reconnect       | Receive | 服务端要求重连         |
| 9   | Invalid Session | Receive | 鉴权失败               |
| 10  | Hello           | Receive | 连接成功，下发心跳间隔 |
| 11  | Heartbeat ACK   | Receive | 心跳确认               |
| 12  | HTTP Callback   | Reply   | HTTP 回调模式回包      |
| 13  | 回调地址验证    | Receive | Webhook 验证           |

### 连接流程

1. GET `/gateway` → 获取 wss 地址
2. WebSocket 连接 → 收到 Hello (op=10): `{"heartbeat_interval": 45000}`
3. 发送 Identify (op=2):
   ```json
   {
     "op": 2,
     "d": {
       "token": "QQBot {access_token}",
       "intents": 33554432,
       "shard": [0, 1],
       "properties": {}
     }
   }
   ```
4. 收到 Ready (op=0, t="READY"): `{"session_id": "...", "user": {...}, "shard": [0, 0]}`
5. 心跳循环: 每 `heartbeat_interval` ms 发送 op=1 (带最新 `s`)
6. 收到事件 Dispatch (op=0, t="事件类型")

### Intents 事件订阅

| 名称                  | 位值    | 说明                        |
|-----------------------|---------|-----------------------------|
| GUILDS                | 1 << 0  | 频道事件                    |
| GUILD_MEMBERS         | 1 << 1  | 频道成员事件                |
| GUILD_MESSAGES        | 1 << 9  | 频道消息 (仅私域)           |
| GUILD_MESSAGE_REACTIONS | 1<<10 | 消息表情表态                |
| DIRECT_MESSAGE        | 1 << 12 | 频道私信                    |
| GROUP_AND_C2C_EVENT   | 1 << 25 | 群聊+单聊消息               |
| INTERACTION           | 1 << 26 | 互动事件                    |
| MESSAGE_AUDIT         | 1 << 27 | 消息审核                    |
| FORUMS_EVENT          | 1 << 28 | 论坛事件 (仅私域)           |
| AUDIO_ACTION          | 1 << 29 | 音频事件                    |
| PUBLIC_GUILD_MESSAGES | 1 << 30 | 频道@机器人 (公域)          |

### 消息事件: C2C_MESSAGE_CREATE

单聊消息，intents: `1 << 25`。

Payload.d 结构:
```json
{
  "id": "ROBOT1.0_.b6nx.CVryAO0nR58RXuU6SC.m92gc19j02qKqdm8ek!",
  "author": {
    "user_openid": "E4F4AEA33253A2797FB897C50B81D7ED"
  },
  "content": "你好",
  "timestamp": "2023-11-06T13:37:18+08:00"
}
```

### 消息事件: GROUP_AT_MESSAGE_CREATE

群聊@机器人消息，intents: `1 << 25`。

Payload.d 结构:
```json
{
  "id": "ROBOT1.0_eBIyWnxpmSu6uLQ7u7fU0eGloKGYg4eEa737vRyKnMCgyZjKi7JLYkQ9B0VapbiY",
  "author": {
    "member_openid": "E4F4AEA33253A2797FB897C50B81D7ED"
  },
  "content": " 123",
  "group_openid": "C9F778FE6ADF9D1D1DBE395BF744A33A",
  "timestamp": "2023-11-06T13:37:18+08:00"
}
```

### 发送消息

基础 URL: `https://api.sgroup.qq.com`

| 场景       | Method | Path                              |
|------------|--------|-----------------------------------|
| 单聊       | POST   | `/v2/users/{openid}/messages`     |
| 群聊       | POST   | `/v2/groups/{group_openid}/messages` |
| 文字子频道 | POST   | `/channels/{channel_id}/messages` |
| 频道私信   | POST   | `/dms/{guild_id}/messages`        |

请求体 (文本消息):
```json
{
  "content": "消息内容",
  "msg_type": 0,
  "msg_id": "回复的消息ID",
  "msg_seq": 1
}
```

消息发送限制:
- 单聊主动消息: 每月 4 条；被动回复: 60min 内最多 5 次
- 群聊主动消息: 每月 4 条；被动回复: 5min 内最多 5 次

### Event → channel_id 映射

| QQ 事件                | channel_id               |
|------------------------|---------------------------|
| C2C_MESSAGE_CREATE     | `qq:c2c:{user_openid}`   |
| GROUP_AT_MESSAGE_CREATE| `qq:group:{group_openid}`|

## 架构

```
src/
├── lib.rs
├── adapter.rs          # Adapter trait 实现，连接各模块
├── config.rs           # QQConfig { app_id, client_secret }
├── error.rs            # QqError 枚举
├── ws/
│   ├── mod.rs
│   ├── client.rs       # WebSocket 连接管理 + payload 解析
│   └── heartbeat.rs    # 心跳循环
├── api/
│   ├── mod.rs
│   ├── auth.rs         # AccessToken 获取/缓存/自动刷新
│   ├── message.rs      # 消息发送 HTTP 调用
│   └── types.rs        # API 请求/响应结构体
└── event/
    ├── mod.rs
    └── converter.rs    # QQ 原生事件 → 统一 Event 转换
```

## 日志

使用 `tracing` + `tracing-subscriber`。日志等级通过配置文件（支持环境变量展开）控制：

```toml
# config/default.toml
[core]
log_level = "${RUST_LOG:-info}"
```

### 日常使用

```bash
# info 级别（默认，仅关键节点）
cargo run

# debug 级别
RUST_LOG=debug cargo run

# trace 级别（包含原始 WS 消息、心跳等）
RUST_LOG=trace cargo run

# 仅查看 QQ adapter 的 trace 日志
RUST_LOG=botadapt_qq=trace cargo run

# 多模块分等级
RUST_LOG=botadapt_core=warn,botadapt_qq=debug cargo run
```

### Span 层次

每条消息日志自带 span 层级，自动关联上下文：

```
event{event_id=..., channel_id=..., platform=qq}
  ├── plugin{plugin=builtin}
  └── send_message{platform=qq, user_id=..., text=pong!}
        └── send_c2c_message{openid=..., text=pong!}
```

## TODO - 遗留特性

### 事件类型
- [x] 单聊消息 (C2C_MESSAGE_CREATE)
- [ ] 群聊消息 (GROUP_AT_MESSAGE_CREATE)
- [ ] 频道@机器人 (AT_MESSAGE_CREATE, intents 1<<30)
- [ ] 频道私信 (DIRECT_MESSAGE_CREATE, intents 1<<12)
- [ ] 频道全量消息 (MESSAGE_CREATE, intents 1<<9, 私域)
- [ ] 好友添加/删除事件 (FRIEND_ADD, FRIEND_DEL)
- [ ] 机器人入群/退群事件 (GROUP_ADD_ROBOT, GROUP_DEL_ROBOT)
- [ ] 消息撤回事件 (C2C_MSG_REJECT, GROUP_MSG_REJECT)
- [ ] 互动事件 (INTERACTION_CREATE)
- [ ] 审核事件 (MESSAGE_AUDIT)
- [ ] 论坛事件 (FORUMS_EVENT)
- [ ] 音频事件 (AUDIO_ACTION)

### 消息类型
- [ ] Markdown 消息 (msg_type=2)
- [ ] Ark 消息 (msg_type=3)
- [ ] Embed 消息 (msg_type=4)
- [ ] 富媒体消息: 图片/语音/视频 (msg_type=7)
- [ ] 消息引用 (message_reference)
- [ ] 互动召回消息 (is_wakeup)
- [ ] Keyboard 消息按钮

### 连接特性
- [ ] Shard 分片负载均衡
- [ ] Resume 断线重连 + 事件补发
- [ ] Webhook 模式 (签名校验 ed25519 + 回调)

### 其他
- [ ] 屏蔽词/链接白名单配置
