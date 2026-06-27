# 去 Clash / CVR 依赖与品牌脱离计划

## 目标

项目已经从原始 Clash Verge / Clash Verge Rev 形态大量改造，下一阶段目标是把产品身份、发布链路、供应链来源和文档说明逐步收回到个人独立维护体系。

这份文档只定义清理边界和执行顺序，不直接修改代码。

## 当前阶段更新

截至 2026-06-26，以下事项已经落地或主体完成：

- **品牌/元数据（Phase 1 主体已完成）**：`src-tauri/Cargo.toml` 的 `authors` 已为 `["tanzanite2025"]`；`package.json` 的 `name` 已为 `clash-verge-optimized`；app id 已为 `io.github.tanzanite2025.clash-verge-optimized`；README 内核说明已改为「进程内纯 Rust 内核 `learn-gripe`」；模板/配置头部已为 `Clash Verge Optimized`。
- **供应链（Phase 2/3 主体已完成）**：仓库内已无任何 `clash-verge-rev/*` 的 Git 依赖或 release 下载链；`sysproxy`、`clash-verge-logging`、`clash-verge-service-ipc` 均为 `crates/` 下的 workspace 本地 crate（策略 B：vendor）。`scripts/prebuild.mjs` 不再下载外部 latest，sidecar/service/geodata/loopback/SimpleSC 资源已改为本地受控。
- **内核（与本计划相关的上游脱离已完成）**：Go/Mihomo sidecar 已完全退役，`crates/tauri-plugin-mihomo` 已删除，数据面由进程内 `learn-gripe` 承载。详见 [`docs/go-to-rust-migration-roadmap.md`](go-to-rust-migration-roadmap.md)。
- macOS service identity 已集中，现役身份与 legacy cleanup 已分边界
- Windows installer 的 legacy cleanup 已从主安装/卸载流程中抽离
- Linux / macOS 的兼容迁移职责已开始按平台边界收束
- `src-tauri/src/utils/dirs.rs` 中跨平台共享的 legacy path migration 已集中为配置化边界；Tauri 配置兼容边界已收口到 `src-tauri/compatibility-boundaries.md`

因此，本文档里 Phase 1–3 大部分描述应作为「迁移记录」阅读，而非当前待办；仍未做的主要是 **Phase 6**（app id / service name / deep-link 正式改名）及其前置的 legacy 删除窗口（Phase 5）。下文凡是引用已删除路径（如 `crates/tauri-plugin-mihomo/`、`mihomo/Dockerfile`）的条目已不再适用，保留仅作历史上下文。

## 核心原则

1. **先脱离品牌和供应链，再处理兼容痕迹**。
2. **能安全改名的马上改；承担升级迁移职责的先标注 legacy，不直接删**。
3. **避免一次性大重命名**：Rust crate、npm package、app id、deep link、安装器、服务名应分 PR 分阶段处理。
4. **不为了去 Clash 字样破坏用户升级**：老配置目录、旧卸载项、旧快捷方式清理、旧 deep-link 兼容需要保留迁移窗口。
5. **供应链脱离必须可验证**：fork / mirror 后要固定 commit、release tag、sha256 或自建发布资产。

## 当前残留类型

### 1. 真实供应链残留

这些不是普通文案，而是构建或运行时实际会拉取/依赖的外部源。

当前状态：第一阶段和内核运行时默认下载链已经完成本地化。

- `src-tauri` 的 `sysproxy`、`clash-verge-logger`、`clash-verge-service-ipc` 已改为 workspace 本地 crate。
- `scripts/prebuild.mjs` 已改为校验本地 sidecar、service bundle、geodata、loopback、SimpleSC 资源，不再下载外部 latest。
- 默认 geodata / dashboard URL 已清空；缺本地资源时返回明确错误，只有用户显式配置 URL 才联网。
- ~~`mihomo/Dockerfile`~~ 已不存在（Go/Mihomo 内核与其容器构建链已随 sidecar 一并移除），此条不再适用。

后续原则：新增构建资源必须固定来源、版本和校验，不再引入默认 latest 下载链。

### 2. 产品身份 / 元数据残留

这些会影响包元数据、产品观感或对外解释。

| 文件 | 残留 | 建议 |
| --- | --- | --- |
| ~~`src-tauri/Cargo.toml`~~ | — | **已完成**：`authors = ["tanzanite2025"]` |
| ~~`package.json`~~ | — | **已完成**：`"name": "clash-verge-optimized"` |
| ~~`README.md`~~ | 原始项目和 MetaCubeX/mihomo 说明 | **已完成**：README 已改为当前独立内核说明（`learn-gripe`），不再描述 Mihomo/CVR 派生 |
| ~~`crates/tauri-plugin-mihomo/README.md`~~ | — | **不再适用**：`tauri-plugin-mihomo` crate 已删除 |
| `src-tauri/src/utils/tmpl.rs` | 模板注释仍写 `Clash Verge` | 改为新产品名 |
| `src-tauri/src/config/*.rs` | 生成 YAML 头仍写 `Clash Verge` | 改为新产品名 |
| i18n 文案 | 部分语言仍写 `Clash Verge` | 批量统一 |

### 3. 兼容迁移残留

这些看起来是旧品牌，但承担老用户升级、卸载、数据迁移职责，不能和普通文案一起删除。

| 文件 | 残留 | 当前职责 |
| --- | --- | --- |
| `src-tauri/src/utils/dirs.rs` | `LEGACY_APP_ID = io.github.clash-verge-rev...` | 迁移旧数据目录到新 app id |
| `src-tauri/src/utils/dirs.rs` | `clash-verge-rev-backup` | 迁移旧备份目录 |
| `src-tauri/packages/windows/installer.nsi` | 删除旧 `Clash Verge` exe / shortcut / registry | 升级和卸载清理 |
| `tauri.conf.json` | deep-link `clash` / `clash-verge` | 订阅导入兼容 |
| `src-tauri/src/utils/resolve/scheme.rs` | 识别 `clash` / `clash-verge` scheme | 订阅链接兼容 |

处理方式：保留，但集中注释为 legacy migration，并设定未来删除条件。

### 4. 需要单独确认的系统级标识

| 文件 | 残留 | 风险 |
| --- | --- | --- |
| `src-tauri/packages/macos/entitlements.plist` | `io.github.clash-verge-rev.clash-verge-rev` application group | 可能影响 macOS 签名、权限、group 容器 |
| `src-tauri/packages/macos/info_merge.plist` | `io.github.clash-verge-rev.clash-verge-rev.service` associated bundle | 可能影响 macOS service 关联 |
| service binary 名 | `clash-verge-service*` | 可能影响安装器、权限、IPC、自动启动 |

这类标识不能只做文本替换，必须配合打包测试。

## 分阶段计划

### Phase 0：确定新身份词表

先确定统一命名，否则会出现二次改名。

需要确定：

| 类型 | 当前 | 建议策略 |
| --- | --- | --- |
| 产品显示名 | `Clash Verge Optimized` | 若彻底脱离 Clash，改为新名称 |
| Rust package | `clash-verge-optimized` | 可先保留，或改为新短名 |
| npm package | `clash-verge` | 建议第一批改掉 |
| app id | `io.github.tanzanite2025.clash-verge-optimized` | 若换产品名，需要新 id + legacy migration |
| deep-link | `clash`, `clash-verge` | 建议保留兼容，同时新增新 scheme |
| service 名 | `clash-verge-service` | 后续单独迁移 |
| config 文件注释 | `Clash Verge` | 可安全替换 |

如果新产品名暂未确定，第一批可以只做“去 CVR / 上游链接 / 作者残留”，不碰 app id。

### Phase 1：低风险品牌清理（主体已完成）

目标：清理不会影响运行时和用户升级的文案/元数据。

建议 PR：

```text
docs/chore: remove stale Clash Verge Rev branding references
```

范围：

- [x] `src-tauri/Cargo.toml` authors 已为 `["tanzanite2025"]`。
- [x] `package.json` name 已为 `clash-verge-optimized`。
- [x] README 内核说明已改为个人独立维护内核（`learn-gripe`）。
- [x] ~~`crates/tauri-plugin-mihomo/README.md`~~ — crate 已删除，此项不再适用。
- [ ] config/profile/template 注释：已为 `Clash Verge Optimized`；若最终产品名脱离“Clash”字样需再调。
- [ ] i18n 中纯显示文案：部分语言（如 `src/locales/ru/*`）仍写 `Clash Verge`，待统一。

不包含：

- app id 修改。
- deep-link 删除。
- installer legacy cleanup 删除。
- service binary 重命名。
- Git dependency 替换。

验证：

```text
pnpm typecheck
pnpm format:check
```

### Phase 2：CVR Git 依赖脱离（已完成）

> **状态：已落地。** 仓库内已无 `clash-verge-rev/*` Git 依赖；`sysproxy`、`clash-verge-logging`、`clash-verge-service-ipc` 均已按**策略 B（vendor 到 `crates/`）**落地。下文保留作为当时决策记录。

目标：不再从 `clash-verge-rev/*` 拉 Rust crate。

建议 PR：

```text
chore: vendor or fork CVR-derived Rust dependencies
```

候选策略：

#### 策略 A：fork 到个人账号

把以下仓库 fork 到 `tanzanite2025/*`：

- `sysproxy-rs`
- `clash-verge-logger`
- `clash-verge-service-ipc`

然后修改：

```toml
sysproxy = { git = "https://github.com/tanzanite2025/sysproxy-rs", rev = "<commit>" }
clash_verge_logger = { git = "https://github.com/tanzanite2025/clash-verge-logger", rev = "<commit>" }
clash_verge_service_ipc = { git = "https://github.com/tanzanite2025/clash-verge-service-ipc", rev = "<commit>" }
```

优点：改动小。

缺点：仍是外部 Git 依赖，需要维护 fork。

#### 策略 B：vendor 到 workspace

把代码放进 `crates/`：

```text
crates/sysproxy
crates/service-ipc
crates/app-logger
```

然后改为 path dependency。

优点：完全脱离外部仓库，代码审计更直接。

缺点：初次 PR 较大，需要处理 license、crate name、更新路径。

建议：`logger` 可优先 vendor；`service-ipc` 和 `sysproxy` 如果代码量较大，先 fork 固定 rev，再分阶段 vendor。

验证：

```text
cargo metadata --no-deps --format-version 1
pnpm typecheck
```

Windows Rust 编译环境当前可能因 MSVC linker 阻断，需在可用 CI/本地验证 `cargo check`。

### Phase 3：service bundle 发布源脱离（主体已完成）

> **状态：`scripts/prebuild.mjs` 已不再从外部 release 下载。** 现为校验本地 sidecar/service 资源，仓库内无 `clash-verge-rev/*` 下载链。下文的下载-URL 方案仅作为“若未来改为远程发布”的参考；如仍走本地受控资源则无需实现。

目标：`scripts/prebuild.mjs` 不再从 CVR release 下载 service bundle。

需要先完成：

1. 个人 fork / vendor `clash-verge-service-ipc`。
2. 在本仓库或独立仓库建立 service release。
3. 产物命名、平台矩阵和 `SIDECAR_HOST` 保持兼容。
4. 为下载产物增加 sha256 manifest。

建议替换：

```js
const SERVICE_LATEST_URL =
  'https://github.com/tanzanite2025/clash-verge-service-ipc/releases/latest'
const SERVICE_URL_PREFIX =
  'https://github.com/tanzanite2025/clash-verge-service-ipc/releases/download'
```

更稳妥做法：不要 latest，使用固定版本 + sha256：

```js
const SERVICE_VERSION = 'vX.Y.Z-tanzanite.1'
const SERVICE_SHA256 = {
  'x86_64-pc-windows-msvc': '...',
}
```

### Phase 4：MetaCubeX geodata / dashboard 脱离（主体已完成）

> **状态：默认资源不再指向 MetaCubeX。** 已清空内置远程默认 geodata / dashboard URL；缺本地资源时报错，仅用户显式配置才联网。下述“后续可选决策”（自建 geodata 镜像 / sha256 / 自建 dashboard）仍待定。

目标：默认资源来源不再指向 MetaCubeX。

当前策略：

1. 不再内置任何远程默认 geodata URL。
2. 不再默认下载远程 dashboard。
3. 桌面端由外层构建提供本地受控资源。
4. Docker 镜像由构建上下文提供本地受控 geodata。

后续可选决策：

1. 是否自建 geodata 镜像。
2. 是否为可选远程资源增加固定 release tag 和 sha256。
3. 是否提供本项目自建 dashboard 发布资产。

建议：

- 先把下载 URL 配到常量或配置文件，方便以后替换。
- 下载时引入 sha256 校验。
- README 标明 geodata 来源与独立内核的关系：数据源不是内核来源。

### Phase 5：兼容项标注与删除窗口

目标：让 legacy 痕迹可控，而不是散落在代码里。

`src-tauri` 这层 Tauri 配置兼容边界已单独收口到 `src-tauri/compatibility-boundaries.md`，作为 `tauri.conf.json`、deep-link 注册和 scheme 解析的唯一说明入口。

建议新增注释规范：

```rust
// Legacy migration from Clash Verge Rev.
// Keep until at least v0.x.y so existing users can migrate app data.
```

对这些位置只加注释/集中封装，不立即删：

- `LEGACY_APP_ID`
- `LEGACY_BACKUP_DIR`
- Windows installer registry / shortcut cleanup
- Linux `conflicts/replaces/obsoletes`
- `clash://` / `clash-verge://` subscription scheme

删除条件：

1. 至少发布一个带迁移逻辑的稳定版本。
2. 用户升级路径验证通过。
3. release note 告知下一版本删除 legacy cleanup。
4. 删除 PR 只删 legacy，不混入其他功能。

### Phase 6：app id / service name / deep-link 正式改名

这是最容易破坏升级的阶段，必须最后做。

建议拆分：

1. 新增新 deep-link scheme，保留旧 scheme。
2. 新 app id + 从旧 app id 迁移数据。
3. 新 service name + 旧 service 卸载/停止逻辑。
4. Windows/macOS/Linux 打包分别验证。
5. 一个版本后再考虑移除旧 scheme。

## 推荐的执行顺序

```text
[x] PR 1: 文案和元数据清理
[x] PR 2: plugin README / README / package name / authors（plugin crate 已删除）
[x] PR 3: fork or vendor logger（已 vendor 为 clash-verge-logging）
[x] PR 4: fork or vendor sysproxy/service-ipc（已 vendor 到 crates/）
[x] PR 5: service bundle release source（已改为本地受控，无外部下载）
[x] PR 6: geodata/dashboard source policy（默认已不指向 MetaCubeX）
[ ] PR 7: mark legacy migration blocks with retention window
[ ] PR 8: add new app scheme / new app id migration
[ ] PR 9: service name migration
[ ] PR 10: remove legacy cleanup after migration window
```

> 注：PR 5/6 的 sha256 完整性校验仍为可选后续项；上面打勾仅表示“不再依赖 CVR/MetaCubeX 外部源”。

## 不建议现在做

- 不要直接删除 `LEGACY_APP_ID`。
- 不要直接删除 Windows installer 里的旧注册表/快捷方式清理。
- 不要直接删除 `clash://` scheme。
- 不要一次性重命名所有 crate、service、binary、app id。
- 不要把 geodata URL 从 MetaCubeX 换成未知源但不加 sha256。
- 不要修改 `Cargo.lock` 指向个人 fork，除非 fork 已存在并确认可拉取。

## 第一批安全清理清单

如果下一步要动代码，建议第一批只做：

1. [x] `src-tauri/Cargo.toml` authors。
2. [x] `package.json` name。
3. [x] README 当前内核说明。
4. [x] ~~`crates/tauri-plugin-mihomo/README.md`~~ — crate 已删除，不适用。
5. [ ] YAML/template 文件头部的 `Clash Verge` 文案（当前为 `Clash Verge Optimized`）。
6. [ ] i18n 里用户可见的 `Clash Verge` 文案。
7. [ ] 给 legacy blocks 加注释，但不删除。

这批不需要 fork 外部仓库，风险最低。

## 第二批供应链清理清单

第二批再处理：

1. `clash-verge-logger`
2. `sysproxy-rs`
3. `clash-verge-service-ipc`
4. service bundle release 下载源
5. geodata release 下载源
6. dashboard 默认 URL

这批需要先准备 fork、release、sha256 或 vendor 代码。

## 完成标准

短期完成标准：

- 新文档/README 不再把当前项目描述为 CVR 派生维护版。
- Cargo/npm 元数据不再暴露旧作者/旧包名。
- 新生成配置文件不再写旧产品名。
- legacy 痕迹都有明确注释和删除窗口。

中期完成标准：

- 构建不再从 `clash-verge-rev/*` 拉依赖或二进制。
- 默认下载资源有固定来源和完整性校验。
- macOS/Windows/Linux 包标识统一到新产品身份。

长期完成标准：

- 只保留必要的协议兼容名，例如 `clash://` 作为订阅生态兼容入口。
- 所有 CVR / Clash Verge Rev 残留只出现在历史致谢、license 或 migration note 中。
- Go sidecar 逐步收敛为个人独立内核，最终按 Go → Rust 路线继续替换。
