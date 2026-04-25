# Kernelaw 内核设计草案（v2）

> 定位：**小而稳定的 runtime kernel + Gateway 控制面 + 可插拔扩展体系**。
>
> 架构短句：**Hermes 风格运行时 + OpenClaw 风格 Hub/Gateway**。

## 1. 设计结论（先定方向）

Kernelaw 继续采用 **Gateway 模式**，但将 Gateway 相关能力彻底模块化：

- 内核只负责 runtime（process/event/tape/state/effect）。
- Gateway 只负责 control plane（session/bindings/routing/delivery/sandbox）。
- WebUI、Gateway WS、Browser 自动化能力都做成独立模块，而不是硬编码进 kernel。
- provider/tool/memory/policy 统一按扩展协议接入。

**一句话**：先把内核做小做稳，再在网关侧拼装能力。

---

## 2. 分层边界

### 2.1 Kernel（核心层）

只保留以下原语：

- `Process`
- `EventLog`
- `Tape`
- `StateStore`
- `Scheduler`
- `ModuleRegistry`
- `CapabilityRegistry`
- `Checkpoint`
- `Effect`

### 2.2 Gateway / Hub（控制面）

由 Gateway（或 Hub）长期运行并负责：

- session 生命周期
- bindings（channel/account/peer/thread -> agent scope）
- routing / delivery
- workspace 与隔离策略
- auth profile
- agent 目录管理

### 2.3 Edge Modules（边缘模块）

明确模块化，不进 kernel：

- `webui-module`
- `gateway-ws-module`
- `browser-module`（browser 自动化/网页执行能力）
- `channel-*`（Discord / Telegram / Slack / WeChat ...）
- `mcp-bridge-module`

> 结论：WebUI / WS / Browser 都是 Gateway 的可插拔模块，而非内核职责。

---

## 3. 向 Hermes / OpenClaw 借鉴的设计

## 3.1 Hermes 侧（runtime 相关）

优先借鉴两块：

1. **provider 抽象**：模型调用统一为 provider adapter，避免业务层直接耦合厂商 API。
2. **tool 配置模型**：工具声明、参数 schema、可见性控制与执行约束统一配置化。

目标是把“模型调用”和“工具执行”都编译到标准 `Effect`/`Event` 流中。

## 3.2 OpenClaw 侧（control plane 相关）

优先借鉴三块：

1. **Hub/Gateway 常驻控制面**：多 agent 与渠道接入都在 control plane 处理。
2. **channel 路由模型**：通过 bindings 做稳定路由，而不是在 prompt 层做分流。
3. **scope 隔离**：每个 agent 拥有独立 workspace/session/auth/capability boundary。

---

## 4. 核心对象模型

### 4.1 `Process`

调度单位，表示一个运行中的 agent 实例：

- 当前状态
- 事件输入
- 语义历史引用
- checkpoint 指针
- 当前绑定模块与 capability

### 4.2 `EventLog`

append-only 事实流，记录 runtime 真实发生事件。

### 4.3 `Tape`

面向模型/人类的语义层历史。主类型固定：

- `user`
- `assistant`
- `tool`
- `injection`

细节由 subtype 表达（如 `assistant.final` / `tool.result`）。

### 4.4 `StateStore`

运行态快照，不承载历史。

### 4.5 `Effect`

待执行动作集合（tool/memory/delegate/approval/emit/pause/finish）。

---

## 5. Event 与 Tape 关系

- Event = runtime 事实
- Tape = EventLog 投影出的语义视图

并不是所有事件都进入 Tape。`pause/resume/wake/timeout/approval_pending/checkpoint_created` 等可只保留在 event 层。

推荐 `Event` 字段：

- `family`
- `type`
- `source`
- `payload`
- `durable`
- `visible_in_tape`
- `causation_id`
- `correlation_id`

---

## 6. 运行循环（step lifecycle）

```text
ingest -> reduce -> project -> deliberate -> normalize -> execute -> commit -> schedule
```

- `ingest`：收用户输入、channel 消息、tool 结果、scheduler wake、external callback。
- `reduce`：状态机推进。
- `project`：生成本轮上下文视图（TapeView + memory/policy summary + visible tools + injections）。
- `deliberate`：调用 provider 或其他推理器。
- `normalize`：标准化为 `TapeItem[]` + `Effect[]`。
- `execute`：执行效果动作。
- `commit`：写入 EventLog/Tape/State/Checkpoint/Trace。
- `schedule`：决定继续、等待、暂停、结束。

---

## 7. 扩展机制与 Hook

扩展不直接改内核状态，只允许：

1. 注册 typed hooks
2. 产出 event/effect
3. 贡献 projection patch / capability

建议 hook 生命周期：

- before model resolve
- before prompt build
- before reply
- before tool call
- after tool call
- before message write
- session start/end
- subagent spawning/spawned/ended
- gateway start/stop
- install/policy guard

hook 元信息：priority / merge policy / terminal stop / failure policy / sync-async / claim-modify-observe。

---

## 8. Gateway 模块化蓝图（新增）

Gateway 按“核心 + 模块”组织：

### 8.1 Gateway Core

- agent registry
- session manager
- binding router
- delivery bus
- auth/sandbox gate

### 8.2 Gateway Modules

- `webui-module`：页面交互、调试视图、运行控制。
- `gateway-ws-module`：实时事件/日志推送与控制命令通道。
- `browser-module`：浏览器任务执行、抓取、网页自动化（作为能力模块，不进入 kernel）。
- `channel-*`：不同 IM/社媒渠道适配。
- `mcp-bridge-module`：与 MCP server/client 的协议桥接。

> Gateway core 只编排，不直接实现具体渠道和 UI。

---

### 8.3 模块装配模式（并存：编译装配 + WASM 运行时扩展）

你提到的冲突点可以直接统一成“双轨”：

1. **编译时模块装配（静态）**：像 lijux 一样在构建前弹出 TUI，选择要编译进发行包的模块集合。  
2. **运行时 WASM 扩展（动态）**：在已编译内核/网关上继续加载或卸载 WASM 插件。

两者并不冲突，职责不同：

- 编译时选择：决定“这个二进制内置了哪些高可信模块”，适合网关基础能力和企业发行版裁剪。
- 运行时加载：决定“这个实例当前启用了哪些可热插拔能力”，适合快速试验和第三方生态。

建议规则：

- `core` 永远静态编译（kernel/gateway-core 不可缺失）。
- `builtin modules` 可在 TUI 勾选后静态链接。
- `wasm plugins` 默认从插件目录或 registry 动态加载。
- 同名能力冲突时，按优先级：`policy > builtin > wasm`，并记录冲突事件到 EventLog。

---

## 9. 项目结构建议（支持 TUI 选配与外部模块）

```text
kernelaw/
  crates/
    kernel-core/              # Process/Event/Tape/State/Effect + step loop
    kernel-hooks/             # typed hook runner
    kernel-plugin-abi/        # 通用插件 ABI（Rust/WASM）
    gateway-core/             # Hub/session/binding/router/delivery
    module-sdk/               # 模块开发 SDK（provider/tool/channel/...）

  modules/
    builtin/
      webui-module/
      gateway-ws-module/
      browser-module/
      channel-discord/
      channel-telegram/
    wasm/                     # 预置 wasm 插件（可选）

  apps/
    kernelawd/                # 主守护进程（读取装配清单并启动）
    kernelaw-cli/             # CLI + 构建前 TUI 入口

  manifests/
    module-index.toml         # 可发现模块索引（本地）
    build-profile.default.toml

  external/
    repos/                    # 拉取的外部仓库缓存
    links/                    # 本地目录软链接或映射
```

### 9.1 编译前 TUI（module picker）

`kernelaw-cli build` 时进入 TUI：

- 勾选 builtin 模块（静态链接）。
- 选择是否启用 WASM runtime loader。
- 生成 `build-profile.<name>.toml`（锁定本次构建选择）。
- 输出可复现构建参数（CI 可直接消费 profile，无需 TUI 交互）。

### 9.2 外部仓库 / 本地目录接入

支持两种来源并统一进入 `module-index.toml`：

- `git` 来源：`repo + rev/tag + subdir`。
- `path` 来源：本地目录映射（适合 monorepo 或私有模块）。

解析后分三类：

1. Rust 内建模块（参与编译）
2. WASM 插件源码（构建成 `.wasm`）
3. 纯配置模块（policy/persona/workflow）

### 9.3 与安全边界的关系

- 外部模块默认在“未信任”级别，必须显式批准后进入 build profile。
- WASM 插件按 capability 声明最小授权。
- Gateway 对 channel/browser 类模块额外施加 sandbox 与审计日志。

---

## 10. 最小数据结构（草案）

```rust
pub struct Process {
    pub id: ProcessId,
    pub status: ProcessStatus,
    pub checkpoint: Option<CheckpointId>,
    pub capability_set: CapabilitySet,
    pub module_set: ModuleSet,
}

pub struct Event {
    pub id: EventId,
    pub process_id: ProcessId,
    pub family: EventFamily,
    pub ty: String,
    pub source: EventSource,
    pub payload: serde_json::Value,
    pub durable: bool,
    pub visible_in_tape: bool,
    pub causation_id: Option<EventId>,
    pub correlation_id: Option<String>,
    pub ts: Timestamp,
}

pub struct TapeItem {
    pub id: TapeId,
    pub process_id: ProcessId,
    pub major: TapeMajor, // user | assistant | tool | injection
    pub subtype: String,
    pub content: serde_json::Value,
    pub refs: Vec<EventId>,
    pub ts: Timestamp,
}

pub struct Effect {
    pub id: EffectId,
    pub process_id: ProcessId,
    pub kind: EffectKind,
    pub payload: serde_json::Value,
    pub blocking: bool,
}
```

---

## 11. `step()` 伪代码

```text
fn step(process_id):
  events = ingest(process_id)
  state = reduce(process_id, events)

  projection = project(
    event_log(process_id),
    tape(process_id),
    state,
    capability_view(process_id)
  )

  raw = deliberate(projection)
  (tape_items, effects) = normalize(raw)

  results = execute(effects)

  commit(
    events + results.events,
    tape_items + results.tape_items,
    results.state_patch,
    maybe_checkpoint(state)
  )

  schedule(process_id)
```

---

## 12. 最小插件 ABI

```text
plugin_init(manifest) -> PluginHandle
plugin_capabilities() -> CapabilityDescriptor[]
plugin_hooks() -> HookRegistration[]
plugin_project(ctx) -> ProjectionPatch[]
plugin_apply_effect(effect) -> EffectResult
plugin_shutdown() -> void
```

约束：

- 插件不能直接写 kernel 内部状态。
- 插件副作用必须通过 effect/result/event 回传。
- trace 与 observer 统一走 hook 与事件流。

---

## 13. 实施顺序（按你的要求：先内核再扩展）

### Phase 1：Kernel First（必须先完成）

1. 固化 `Process/Event/TapeItem/Effect` schema。
2. 实现单进程 `step()`。
3. 打通 checkpoint/replay。
4. 落地 typed hook runner。

### Phase 2：基础扩展（Hermes 风格）

1. provider adapter 接口。
2. tool 配置与执行约束。
3. memory/policy 插件。

### Phase 3：Control Plane（OpenClaw 风格）

1. Hub/Gateway core（session/bindings/routing）。
2. channel 路由与 agent scope 隔离。
3. 交付总线与权限边界。

### Phase 4：装配系统与模块供应链

1. `kernelaw-cli build` 的 TUI 选配流程
2. `build-profile.toml` 锁定与 CI 非交互复现
3. 外部 repo / 本地 path 模块接入与签名校验

### Phase 5：Gateway 模块化能力

1. `gateway-ws-module`
2. `webui-module`
3. `browser-module`
4. 其他 channel / MCP bridge 模块

---

## 14. 当前评审意见

你这版思路是对的，关键升级点就是：

- 继续坚持 Gateway 模式；
- 把 WebUI / WS / Browser 从“内建功能”改成“模块化能力”；
- Runtime 借 Hermes（provider + tool config）；
- Control Plane 借 OpenClaw（Hub + channel + scope）；
- 严格按“内核先行，扩展后置”的节奏推进。

这版可以作为接下来实现的正式基线，且“编译时模块装配 + 运行时 WASM 扩展”可以并存，不冲突。
