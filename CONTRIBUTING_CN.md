# 贡献指南

[English](./CONTRIBUTING.md)

感谢你对 OpenBitFun 的兴趣！OpenBitFun 是一个由 Rust 与 TypeScript 驱动的多端 AI 编程环境，桌面端/CLI/Server 共享核心逻辑。本指南说明如何高效参与贡献。

## 行为准则

请保持尊重、友善与建设性沟通。我们欢迎不同背景与经验的贡献者。

## 快速开始

### 环境准备

- Node.js 22.12+（建议 LTS 版本）
- pnpm 10.15.0（建议通过 Corepack 使用）
- Rust toolchain（通过 rustup 安装）
- 桌面端开发需准备 Tauri 依赖

OpenBitFun 将本地 JavaScript 构建和 CI 统一到 Node.js 22.12+。仓库里的
GitHub Actions 升级使用的是兼容 Node.js 24 的 action runtime，但项目脚本
默认仍以 Node.js 22.12+ 为基线，除非局部指南另有说明。从旧 Node.js 版本切换
后，请重新运行 `pnpm install`。

#### 构建前置检查

当 `cargo check --workspace`、`cargo check -p openbitfun-desktop` 或 pnpm 构建
命令报出难以理解的错误（如 "resource path doesn't exist" 或 sherpa-onnx
下载失败）时，运行前置检查以识别缺失的依赖并获取可操作的修复命令：

```bash
pnpm run check:build-prereqs           # 仅检查
pnpm run check:build-prereqs -- --fix  # 尝试自动修复缺失的前置依赖
```

检查项包括：

- 缺少 `node_modules`（修复：`pnpm install`）
- 缺少 `src/mobile-web/dist`（修复：`pnpm run prepare:mobile-web` —
  openbitfun-desktop 的 Tauri 构建脚本将该目录作为资源引用，缺失时
  `cargo check -p openbitfun-desktop` 和 `cargo check --workspace` 会失败）
- 缺少 sherpa-onnx 预编译库（sherpa-onnx-sys 构建脚本会在构建时从
  GitHub 下载；若网络连通性差导致下载失败，设置
  `SHERPA_ONNX_LIB_DIR` 指向 `target/sherpa-onnx-prebuilt/` 下的预编译
  lib 目录以使用本地副本）

### 安装依赖

```bash
pnpm install
```

pnpm 工作区统一使用根目录的 `pnpm-lock.yaml`，不要为工作区包提交
`package-lock.json`。独立的 `packages/dsh-acp` npm 包与
`src/apps/extension-host` Bun 宿主保留各自的锁文件和准备命令。
Tauri 生成的 `gen/` 目录以及本地实施过程记录不进入 Git。

### 常用命令

```bash
# Desktop（日常开发推荐）
pnpm run desktop:dev                # 完整热更新：Vite HMR + Rust 自动重编译并重启
pnpm run prepare:dsh-profile        # 可选：编译 DeepSeek Harness 桥，供本地会话使用

# Desktop（轻量预览，无 Rust 自动重编译）
pnpm run desktop:preview:debug      # 复用预构建二进制 + Vite HMR；Rust 改动需手动重启

# Desktop（生产构建）
pnpm run desktop:build

# E2E
pnpm run e2e:test
```

> **`desktop:dev` 与 `desktop:preview:debug` 的区别**：`desktop:dev` 运行 `tauri dev`，提供**完整热更新** — 前端改动通过 Vite HMR 即时生效，Rust/后端改动会触发增量重编译并自动重启应用，是日常开发的首选方式。`desktop:preview:debug` 启动预构建的 debug 二进制和 Vite dev server；前端编辑仍可 HMR，但 **Rust 侧改动不会自动重编译** — 需要手动停止并重新运行命令（或使用 `--force-rebuild`）。适合仅需迭代前端代码、或希望跳过 `tauri dev` 初始化以更快冷启动的场景。

> 完整脚本列表见 [`package.json`](package.json)。agent 专用命令、验证与架构规则见 [`AGENTS.md`](AGENTS.md)。

### 桌面端调试工具

桌面端 dev 构建会启用 `devtools` Cargo feature。`F12` 打开原生 webview
DevTools；`Cmd/Ctrl + Shift + I` 切换 OpenBitFun 元素检查器，`Cmd/Ctrl + Shift + J`
也可以打开原生 DevTools。面向最终用户的 `release` 构建不会启用这些工具。

## 代码规范与架构约束

架构敏感规则、模块边界和验证矩阵以 [`AGENTS.md`](AGENTS.md) 为准。面向贡献者只需把握：

- 日志只使用英文，并保持必要、可读。
- 用户可见文案走项目 i18n 流程；不要把 Web UI locale catalog 共享给较小产品形态。
- shared core 必须保持平台无关；Desktop/Tauri 细节属于 app adapter，并通过类型化能力接口回流；需要事件投递时使用已有生产 transport adapter。
- Tauri command 使用 `snake_case` 命令名和结构化 `request` 参数。
- core 拆解、feature 边界、依赖边界和构建提速重构必须遵循
  `docs/architecture/product-architecture.md`。
- 功能级规则应放在离代码最近的模块 `AGENTS.md` 中。

## 重点关注的贡献方向

1. 贡献好的想法/创意（功能、交互、视觉等），提交 Issue
   > 欢迎产品经理、UI 设计师通过 PI 快速提交创意，我们会帮助完善开发
2. 优化 Agent 系统和效果
3. 对提升系统稳定性和完善基础能力
4. 扩展生态（Skills、MCP，或者对某些垂域开发场景的更好支持）

## 贡献流程与 PR 约定

### 除功能/修复外的贡献方向

我们欢迎不仅限于功能或修复的 PR。示例包括：

| 贡献方向 | 位置/文件 | 示例说明 |
| --- | --- | --- |
| Prompts | `src/crates/assembly/agent-content/prompts/agents/` | 新增或优化内置提示词；选择逻辑与运行时策略仍由现有模块负责 |
| Tools | `src/crates/assembly/core/src/agentic/tools/implementations/`、`src/crates/assembly/core/src/agentic/tools/registry.rs` | 新增工具实现，并在工具注册表中注册 |
| Subagents | `src/crates/assembly/core/src/agentic/agents/definitions/`、`src/crates/assembly/core/src/agentic/agents/registry/` | 新增子代理定义，并在所属注册表中注册 |
| 模式贡献 | `src/crates/assembly/core/src/agentic/agents/definitions/`、`src/crates/assembly/agent-content/prompts/agents/`、`src/web-ui/src/locales/` | 同步模式策略、内置提示词及所属界面文案 |
| Playbook 与场景指南 | `src/shared/interactive-capabilities/catalog.json`、所属应用 README | 维护源目录并运行 `pnpm run capabilities:generate`；`website/` 消费生成的目录 |

### 开始前

- 先开 Issue 说明问题或方案，尤其是较大改动，以避免重复与设计冲突
- 新功能或 UI 变更建议先讨论设计方向，确保符合产品体验
- 将 Issue 和 PR 模板作为填写指引；保持 PR 聚焦，必要时说明跳过了哪些验证以及原因。
- 较大或风险较高的改动请尽早确定评审深度：风险等级与审批数量见 [代码评审](#代码评审)。

### PR 标题与描述

建议使用 Conventional Commits 风格，便于维护版本记录与自动化流程：

- `feat:` 新功能
- `fix:` 修复问题
- `docs:` 文档变更
- `chore:` 维护/依赖
- `refactor:` 重构且不改行为
- `test:` 测试相关

UI 改动请附前后对比截图或短录屏，方便快速评审。

如为 AI 辅助产出，请在 PR 中注明并说明测试程度（未测/轻测/已测），便于评审风险。

不要提交临时 AI prompt、本地绝对路径、生成的草稿文件、配对密钥、token、证书或无关产物。PR 应聚焦于本次产品或维护改动。

Repository Object Sizes 检查会拒绝超过 5 MiB 的 Git 文件对象，包括在中间提交中加入、
最终又删除的文件。构建产物应放在 Git 仓库之外。现有内置中文字体在
`scripts/git-object-size-policy.json` 中按精确对象 ID 列出例外，修改例外需要审查。
提交前可运行 `node scripts/check-git-object-sizes.mjs --base origin/main --head HEAD`，
省略 `--base` 则检查全部可达历史。检查器的验证命令为
`node --test scripts/check-git-object-sizes.test.mjs`。

### 分支管理

**`main` 分支为默认协作分支，并接受特性 PR。** 本仓库欢迎产品经理、开发者使用 AI 生成代码进行快速验证或提交想法，因此 **所有 PR 请直接提交到 `main` 分支**。

### 变更范围

保持 PR 小而聚焦，避免混杂无关改动。

## 测试与验证

按改动文件和行为选择最小检查。完整构建和大范围测试由 CI 保护；只有改动影响构建、打包、发布行为，
或 CI 无法覆盖对应路径时，才在本地运行更重命令。

常见本地检查：

| 改动类型 | 常用验证 |
| --- | --- |
| 仓库元信息或 GitHub 配置 | `pnpm run check:repo-hygiene && pnpm run check:github-config && git diff --check` |
| 前端运行时或 UI | `pnpm run check:web`；行为变化时再加最近的 focused test |
| Mobile web | `pnpm --dir src/mobile-web run type-check` |
| Rust 共享 runtime 或 services | 遵循最近模块的 `AGENTS.md`，选择单个包/测试目标和最小所需 feature |
| Desktop/Tauri 集成 | `cargo check -p openbitfun-desktop` |
| i18n 资源或契约 | 使用 `AGENTS.md` 中匹配的 i18n 验证行 |

UI 改动在有帮助时附截图或短录屏。无法运行相关检查时，在 PR 中说明原因，并提供风险更低的手动验证路径。

`pnpm run check:web` 会组合执行 Web UI 类型检查，以及 CI 针对前端改动运行的 Appearance contract、
主题颜色和主题视觉治理门禁。

## 代码评审

评审是分层进行的，而不是最后一道闸门。大部分阅读由机器完成，人的时间只花在风险上。
[`REVIEW.md`](REVIEW.md) 是评审方的契约：本仓库的失败形态、发现项如何书写，以及保证评审收敛的限额。
评审前请先阅读它，邀请他人评审前同样请先阅读它。

### 打开 PR 之前

- 给 Agent 可以自行运行的检查，并把命令与输出粘贴到 PR 的 Verification 区块。
  "完成了" 不是证据，命令和它的输出才是。
- 先用新上下文对自己的 diff 跑一遍 Agent 评审：`code-review` skill（Standards 对照
  `CONTRIBUTING.md`、`REVIEW.md` 和最近的 `AGENTS.md`；Spec 对照 `.scratch/` 下的 Issue），
  再加上上表中的 focused 检查。先修掉它发现的问题，再占用人的注意力。
- 保持改动小。容易回滚的改动才容易通过评审。

### 分层

| 层 | 运行内容 | 是否需在人看之前完成 |
| --- | --- | --- |
| 1. 自我验证 | PR Verification 区块中的确切命令与输出 | 是 |
| 2. Agent 评审 | 用新上下文审查 diff；每条发现都要修复或给出理由后关闭 | 是 |
| 3. 自动检查 | CI：构建、测试、仓库卫生、GitHub 配置、核心边界、i18n 与主题审计 | 是 |
| 4. 人工审批 | 按下方风险等级分层 | 中/高风险需要 |
| 5. 合并与回滚 | 小 PR、一步回滚；不便回滚的改动使用 feature flag | 之后 |

### Open Code Review

第二层可以使用 [Open Code Review](https://github.com/alibaba/open-code-review)
（`ocr`）运行：它由确定性流水线加 LLM agent 组成，产出行级发现。它是可选项，
`code-review` skill 与新上下文 subagent 依然是填充第二层的有效方式。

```bash
npm install -g @alibaba-group/open-code-review
ocr config provider      # 或非交互式：ocr config set provider <name>
ocr config model
ocr llm test             # 必须先通过，评审才有意义
ocr review --from main --to <branch>
ocr review --preview     # 只列出将要读取的文件，不消耗模型调用
```

- 凭据保存在仓库之外的 `~/.opencodereview/config.json`（权限 `600`）。永远不要把
  端点 URL 或 API key 写进会被提交的文件——`pnpm run check:repo-hygiene` 会拒绝它们，
  而进入 Git 的密钥必须轮换。
- 在 OpenCode 中，`.opencode/plugins/` 注册了 `ocr_review`、`ocr_health`、
  `/ocr-review` 和 `/ocr-health`。需先在 `.opencode/` 内执行 `npm install`；
  其 `node_modules` 已被 gitignore。
- `ocr` 的发现与其他评审遵循相同的限额与严重度标签：每条发现都要回应，nit 有上限，
  重复问题修在源头而不是重复发布。

### 风险等级

等级由改动本身决定，而不是由作者决定。请在 PR 模板中声明。

| 风险 | 示例 | 人工审批数 |
| --- | --- | --- |
| 低 | 文档、文案、内部专用 helper、flag 后的增量改动、生成物刷新 | 前三层为绿即可，无需人工 |
| 中 | 常规产品改动；缓存、适配器、会话、设置、远程与对端路径 | 一人 |
| 高 | 认证、凭据、发布与打包、CI 工作流、持久化数据结构、升级兼容、安全边界 | 两人 |

- Agent 的批准永远不能算作一个人的批准。
- 指挥 Agent 完成改动的人，等同于必需评审人之一，如同代码由其亲手写成。
- 一次批准只覆盖一个 commit。新 commit（尤其是 Agent 推送或人工解决的合并）需要重新批准。
- 分支保护与 `CODEOWNERS` 是在服务端强制第 3、4 层的手段。在配置之前，风险等级只是作者与
  评审人共同遵守的规范。

### 收敛

Agent 作者与 Agent 评审人可能无限地互相消耗。以下规则用于终止它：

- **每条发现都必须被回应**——修复，或给出简短理由后关闭。沉默永远不会关闭一条发现。
- **评审轮次上限为三轮。** 三轮评审与修复之后，停下来重新审视方案，而不是开始第四轮。
- 首轮之后的再评审 **只读新增 diff**。
- **汇报问题的类别**，让一次修复关掉全部实例；已发布过的发现不再重复发布。

### 把重复评论修在源头

同一条发现在第二个 PR 再次出现时，修复方式不是第三条评论，而是把它写进会在下一次评审之前运行的东西：

- 约定 → [`REVIEW.md`](REVIEW.md) 或最近的 `AGENTS.md`
- 机器能检查的 → lint 规则、审计，或 `scripts/check-*.mjs`
- 已过期的说明 → 它所矛盾的那行文档
- 已经上线的缺陷 → `REVIEW.md` 中的失败形态，以便下次评审能拦住这一类问题

回滚或逃逸的回归是评审证据，不只是事故：请记录失败形态，让下一次评审捕获它。

## 安全与合规

- 不要提交密钥、Token、证书或任何敏感信息
- 新增依赖请确认许可证兼容并说明用途

## 感谢

每一份贡献都很重要，欢迎提交 Issue、PR 或建议！
