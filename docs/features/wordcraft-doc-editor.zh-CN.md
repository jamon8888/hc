# WordCraft 文档编辑器

WordCraft（上游 [storytold/wordcraft](https://github.com/storytold/wordcraft)，MIT/Apache-2.0）
是 OpenBitFun 的 Word/docx 编辑器。Agent 会在用户提示下编辑文档；修订（tracked changes）
始终以 `OpenBitFun Agent (<session>)` 署名。设计决策的原始记录位于
`.scratch/wordcraft-doc-editor/`（map + ticket Answer）；本文仅沉淀稳定的产品契约。

## 能力记录（唯一事实来源）

`openbitfun_services_core::wordcraft::WordcraftProbe`（按需启用 `wordcraft` feature）产出唯一的
`WordcraftCapability` 记录，解析顺序为 **显式配置路径 → 系统 PATH → 托管组件**
（`wordcraft` 组件，版本由 `PINNED_WORDCRAFT_VERSION` 固定）。状态：
`available | unsupported | not-installed | error`；`message`/`remediation` 是可执行的用户文案；
`recheck()` 触发重新探测。所有表面（Tauri 命令、Web 适配器、Agent 工具指引）都**逐字引用该记录**，
不在各自表面重新推导，也绝不伪造可用性。

分发采用上游固定版本的发布包，并以 `SHA256SUMS.txt` 校验（不 vendor 源码，不接受未校验二进制）。
`wordcraft-web` 静态包（COOP/COEP，无控制通道）保留给后续同源嵌入升级；接入前需完成 live probe。

## 编辑器表面（桌面）

- `wordcraft_status_command(recheck)` —— 返回上述记录；支撑状态卡片与 Recheck 操作。
- `wordcraft_open_document_command({ path?, workspaceId? })` —— 以独立 OS 窗口拉起上游 egui 应用：
  - 路径解析走共享桌面解析器（支持工作区相对路径与 runtime URI）；**远程目标响亮拒绝**
    （"local workspace only …"，不静默本地化）。
  - 第一阶段仅 `.docx`，其他格式给出明确拒绝原因。
  - 启动参数带空闲回环控制端口（`--control`）与隔离的 `WORDCRAFT_PROFILE_DIR`
    （`user data/wordcraft/profiles/<stem>-<fingerprint>`）；每次启动的控制密钥
    （`control-key.local-<port>`）只存放在该 profile 中，永不写日志、永不暴露给 Agent。
  - 编辑器**不在托管子进程树内**：它是面向用户的窗口，生命周期长于 OpenBitFun；应用退出
    不得杀掉用户的编辑器。
  - 仅有 `wordcraft-cli` 的安装会拒绝弹窗（提示 Agent 编辑仍可用），可见编辑器需要 `wordcraft` 应用。
- 注册表立场（`remote_surface/table.rs`）：两个命令均为 `LocalOnly`、`ControllerLocal`；
  open 命令在 CLI 对端上为 `REFUSED`，status 为 `CLI_NOT_IMPLEMENTED`。
- Web UI：`.docx` 右键"在 WordCraft 中打开"；"新建 WordCraft 文档"委托其自家起始页/模板；
  **当记录为 `available` 时，`.docx` 默认打开路由到 WordCraft**（`FileTabManager` 单一分流点；
  显式"以文本打开"可绕过；其他记录状态以记录原因响亮提示，绝不回退到二进制乱码查看器）。
  并发编辑按决策采用"最后写入者胜"，无互锁。

## Agent 编辑通道

路由（`mcp::server::wordcraft_session::wordcraft_route_session`）执行单一写者规则：

- **Headless（主通道）**：`wordcraft-cli mcp --author "OpenBitFun Agent (<session>)"
  --automation-read-root <workspace> --automation-write-root <workspace>`，隔离 profile，
  cwd=工作区。写根之外的保存被上游自动化门禁拒绝（可读错误、文档不变）。
  空闲回收 `wordcraft_session_idle_reap_due`（10 分钟）适用于桌面 headless 会话。
- **Live 桥**：文档在可见应用中打开且控制通道可达时，桥以 `mcp --connect 127.0.0.1:<port>`
  运行并携带 `WORDCRAFT_CONTROL_KEY`（仅回环；密钥读取自会话 profile）。
  文档开着而桥不可达时，路由**响亮拒绝**，绝不同另一个写者竞速。
- `document_open_in_visible_app` 是桌面适配器驱动路由的判断输入。

### Agent 工具指引（上游 MCP 契约要点）

- MCP 仅暴露 16 个工具；约 270 条编辑命令经通用 `execute`（`{command, params}`）与
  `batch`（`{commands:[...], keepGoing}`）执行。点命令 ID 以 `list_commands` 为准
  （如 `select.text {text, occurrence}`、`format.bold {value}`）。
- `type_text` 不会按 `\n` 分段——必须传 `{text, paragraphs: true}` 或显式 `text.newParagraph`。
- 始终传 `review.trackChanges {value: true}` —— 裸开关式调用可能把修订**关闭**。
- 失败的命令不会改动文档（脏状态下同样可以"先写后核"）；用 `inspect_document` 与
  独立回读（`wordcraft-cli text <docx>`）双重验证。
- 第一阶段不接入 Zotero 类文献库路径。

## 远程场景

- **远程工作区主机**（SSH/跳板/容器）：设计见 effort map（ticket 14）并经二次研讨论修订——
  供应（provisioning）在控制器侧拉取固定版本的目标平台包（SHA256SUMS 校验），经工作区传输
  （flashgrep 两段式）放入主机托管 POSIX 运行根（`wordcraft/<version>/` + `current`），
  解析阶梯经连接重放并产出同一 `WordcraftCapability` 词汇。供应在会话/任务开始时自动触发
  （可见、响亮）。进程模型（修订后）：**主机上按用户按工作区常驻单守护进程**——附加/分离
  时重验身份（固定版本、roots、author），断连与应用重启不回收，远程不做空闲回收。
  远程宿主能力落地前，桌面命令对远程路径响亮拒绝，绝不回退为本地编辑远程内容。
- **远程控制**（移动端/IM 机器人）：headless 编辑走主机路径；可见窗口按 `ControllerLocal`
  立场给出明确的 unsupported 回复。
- **离线派发（Detached Dispatch）**：docx 编辑任务需要协商到的 wordcraft 能力通告
  （dispatch `supports_provisioning` 先例），绝不假设；缺失时在最前拒绝。
- POSIX 纪律：远程 roots 是 POSIX 字符串，只用远程拼接/转义助手组装；任何客户端侧
  `std::path` 语义都不适用。

## 验证

按所有者选择，不做大而全：

- Rust 焦点测试：`cargo test -p openbitfun-desktop --lib wordcraft`（表面），
  `cargo test -p openbitfun-services-integrations --no-default-features --features mcp --lib
  mcp::server::wordcraft`（会话与路由），`cargo test -p openbitfun-services-core
  --no-default-features --features wordcraft --lib wordcraft`（记录/探测/校验）。
- 注册表：`cargo test -p openbitfun-desktop --lib remote_workspace_policy`；治理：
  `pnpm run check:core-boundaries`、`pnpm run fmt:rs`。
- **设备门（按需，需要真实二进制）**：
  `node tests/e2e/scripts/wordcraft-doc-roundtrip.mjs [--cli <path>] [--strict]` ——
  headless 全环：initialize → 经 `execute`/`batch` 的修订编辑 → 保存 → OOXML 断言
  （`w:ins` 署名 + 粗体 run）→ 独立 `text` 回读 → 写根外保存拒绝。二进制不可解析时响亮拒绝运行。
- 欠硬件验证（当前机器不具备）：真实 GPU 表面下的可见应用控制通道（`wordcraft --control`、
  密钥认证、`engine.execute`、`screenshot`）以及 wordcraft-web 同源嵌入 live probe，
  需在 macOS/Windows 实机完成后再声称可见链路已验证。

## 范围之外

- 不改动、不 fork 上游 WordCraft；以洁净-room 版本消费。
- 不跨工作复用工具命名入口（univer 拥有表格/演示；WordCraft 拥有 `.docx`，
  重叠部分经决策归 WordCraft）。
- 不追逐与 Word 的功能对齐（上游路线图自决）。
