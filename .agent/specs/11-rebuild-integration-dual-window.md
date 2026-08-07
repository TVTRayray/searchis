# SPEC-11 重建前端接入与双窗口形态落地

## 基本信息

- 当前状态：`in_qa`（Esc 关闭行为已获 Orchestrator 人工确认，剩余 Ctrl+Enter/Ctrl+N/管理 CRUD 待真实用户验收）
- 关联阶段：Phase 4（重建整合）
- 当前责任角色：`QA`（剩余 retest 项）
- 关联 PRD：`FR-PCK-01~03`、`FR-MGT-01~03`、`FR-SCH-01~03`、`FR-SNP-03`、`AC-01/05/08/13`、`E-06`
- 前置 Spec：`SPEC-01`（done）、`SPEC-02`（done）
- 允许修改：`src/**`、`src-tauri/**`（窗口创建与命令暴露）、`tauri.conf.json`、`package*.json`、本 spec 与 `master_plan.md`
- 禁止修改：`docs/prds/**`、`storage.rs` 的 Schema/加密/事务/校验逻辑（SPEC-01 已验收，只允许新增只读/粘贴命令的薄封装）

## 背景

重建前端（astryx 组件库、ManagerWindow/SettingsModal/OnboardingWizard/HeaderBar/KeyboardShortcutsModal、rofi 风格 QuickSearchWindow）已由 Master 文件级覆盖到仓库根目录 `src/`，`npm run build` 通过。但该重建处于 mock 形态：检索走 `utils/searchEngine.ts`、数据来自 `data/initialSnippets.ts`，ManagerWindow 等组件不调 IPC。当前 Rust 后端为 SPEC-01/02 已验收状态（SQLCipher 存储、search_index、clipboard.rs 均已实现且测试通过），前端与后端尚未接通。

同时，PRD 定义的应用形态是两个独立窗口：**检索窗口**（全局快捷键呼出的无边框置顶小窗口，rofi/krunner 式，Enter 复制/粘贴后关闭）+ **管理窗口**（常规主窗口，CRUD/筛选/设置）。当前 tauri.conf.json 只有一个常规窗口，重建前端把 QuickSearchWindow 做成单窗口内模态弹出层——视觉近似但非 OS 级独立窗口。本 spec 必须落地双窗口形态。

## 目标与成功标准

- [x] 检索窗口成为独立 Tauri 窗口：无边框、置顶、窄小尺寸（rofi/krunner 式）、Esc 关闭、输入框自动聚焦。
- [x] 管理窗口为独立常规窗口，包含 ManagerWindow/设置/向导视图；两个窗口互不嵌套。
- [x] QuickSearchWindow 从真实后端检索：新增 `search_snippets` 命令（复用 search_index.rs），前端不再依赖 mock searchEngine 与 initialSnippets。
- [x] 复制/自动粘贴回调接真实命令：新增 `copy_snippet` 命令（复用 clipboard.rs 与操作幂等），剪贴板失败显示错误且不计数。
- [x] 管理窗口 CRUD 接 `snippet_create/update/get/list`（契约已一致），revision 冲突与字段错误可显示。
- [x] 敏感片段遮挡、错误码展示与"原 front-baseline 无音效"约束不被重建破坏。
- [x] `cargo test` 既有测试保持通过；`npm run build` 通过；`npm run tauri:build` 可产出可执行文件。

## 非目标

- 不实现 KGlobalAccel 全局快捷键与 X11 自动粘贴（SPEC-03）。
- 不实现设置持久化到 Rust（SPEC-06；重建中的 localStorage 主题逻辑在本 spec 保留即可，SPEC-06 迁移）。
- 不实现回收站、导入导出、向导完整流程（SPEC-05/07/08）。
- 不修改数据库 Schema、加密、事务与校验逻辑。

## 接口契约

| 命令 | 请求 | 成功返回 | 错误码 | 说明 |
|---|---|---|---|---|
| `snippet_list/get/create/update` | 既有契约 | 既有 | 既有 | 已存在，前端直接调用 |
| `search_snippets`（新增） | query, limit | items, total | `DB_OPEN_FAILED` | 薄封装 search_index.rs，仅未删除片段 |
| `copy_snippet`（新增） | id, operationId, keepOpen | copied 统计 | `CLIPBOARD_WRITE_FAILED`,`DB_WRITE_FAILED` | 薄封装 clipboard.rs，operationId 幂等 |

- 新增命令只允许调用已验收的 service/storage 能力，不得重写排序、事务或加密。
- 前端 UI 不得直接拼 SQL 或调用未封装平台命令。
- 检索窗口 `onClose` 对应真实窗口关闭；`Ctrl+E`/`Ctrl+N` 跳转管理窗口必须真实切换窗口并传递预填数据（用 Tauri 窗口间事件或先关后开 + 参数传递，二选一，需记录决策）。

## 失败场景

| 场景 | 预期 | 错误码 | 数据 |
|---|---|---|---|
| 数据库未初始化/密钥不可用 | 错误页显示，不进入 mock 数据 | `DB_KEY_UNAVAILABLE`,`DB_OPEN_FAILED` | 不写入 |
| 剪贴板写入失败 | 检索窗口保留并显示错误，不计数 | `CLIPBOARD_WRITE_FAILED` | 不更新 |
| IPC 契约字段不一致 | 前端显示错误，不崩溃 | `IPC_FAILED` | 不写入 |
| revision 冲突 | 提示重新载入（复用既有逻辑） | `REVISION_CONFLICT` | 不覆盖 |

## 测试与验收

- [ ] `npm run build`、`cargo test`、`cargo clippy`、`npm run tauri:build` 全绿。
- [ ] 两个独立窗口形态在 Arch/KDE/X11 下实机验证（检索窗口无边框置顶、Esc 关闭、聚焦）。
- [ ] 检索、复制、管理 CRUD 走真实数据，无 mock 残留（`grep -r initialSnippets src` 为空或仅测试数据）。
- [ ] 敏感正文不出现在默认可见文本与 a11y label。
- [ ] 手动回归 AC-01、AC-05、AC-08、AC-13。

- [ ] 两个独立窗口形态在 Arch/KDE/X11 下实机验证（检索窗口无边框置顶、Esc 关闭、聚焦）。
- [ ] 检索、复制、管理 CRUD 走真实数据，无 mock 残留（`grep -r initialSnippets src` 为空或仅测试数据）。
- [ ] 敏感正文不出现在默认可见文本与 a11y label。
- [ ] 手动回归 AC-01、AC-05、AC-08、AC-13。

## 实施记录

- Changed Files：`src-tauri/tauri.conf.json`（双窗口：main 常规 + search 无边框置顶隐藏）、`src-tauri/capabilities/default.json`（windows 增加 search）、`src-tauri/src/model.rs`（ManagerRequest）、`src-tauri/src/commands.rs`（open/close_search_window、open_manager_window、take_manager_request + manager_request 存储）、`src-tauri/src/lib.rs`（注册新命令、初始化请求存储）、`src/api/snippets.ts`（search/copy/prepareNew + windowApi）、`src/App.tsx`（窗口标签路由 + 真实 CRUD + 请求消费）、`src/components/QuickSearchWindow.tsx`（真实后端检索/复制、元数据预览、无边框窗口形态）、`src/components/ManagerWindow.tsx`（editRequestId/prefillCreateKey 消费、Snippet 构造补全、异步保存选中回退）、`src/components/HeaderBar.tsx`（快速检索按钮改为打开独立窗口）、`src/components/SettingsModal.tsx`（移除音效开关）、`src/types/snippet.ts`（与 PersistedSnippet 对齐）；删除 `src/data/initialSnippets.ts`、`src/utils/searchEngine.ts`。
- Commands Run：`cargo fmt --all -- --check && cargo test`（32 passed）、`cargo clippy --all-targets -- -D warnings`、`npm run build`（1619 modules）、`npm run tauri:build`（Built application）、release 二进制隔离 XDG 烟测（进程存活、仅主窗口注册、密钥 0600、无 panic）。
- Validation Output：后端 32 tests 全部通过（含 SPEC-01/02 既有测试未回归）；前端 TypeScript/Vite 构建通过；发布二进制启动后 `wmctrl` 基线对比仅新增 "Searchis" 主窗口，检索窗口 `visible:false` 未注册（隐藏启动正确）。
- Residual Risks：窗口显示/聚焦、Ctrl+E/Ctrl+N 跨窗口跳转、输入法组合态为 GUI 行为，需 QA 实机验证；回收站/导入导出/重置按钮暂以提示占位（SPEC-05/07）；设置持久化仍为 localStorage（SPEC-06）；KeyboardShortcutsModal/OnboardingWizard 中残留 macOS 键位与"已授权"假状态属重建基线残留，由 Master 决定是否随 SPEC-06/08 清理。
- Implementation Notes：跨窗口数据传递采用 Rust 侧 `Mutex<Option<ManagerRequest>>` 存储 + `take_manager_request` 消费（管理窗口挂载与重新聚焦时各消费一次）；检索窗口 DTO 不含 content，预览仅展示元数据；音效开关已移除并替换为 PRD 约束说明。
- 审查修复（独立 reviewer + QA 打回后 Coder 修复，2026-08-07/08）：
  - RF1：搜索窗口 X_SetInputFocus BadMatch → 在 `open_search_window` 中先 `set_size` + `center` 再 `show`（强制 GTK 按配置尺寸映射），然后延迟 200ms 再 `set_focus`（等待窗口映射完成后再发 XSetInputFocus，避免对未映射窗口触发 BadMatch）。
  - RF2：`window.show()` 不生效（窗口停留 10×10）→ 同上：`set_size(780,500)` → `center()` → `show()` 序列，保证首次及重复呼出均恢复正确几何。
  - H1：J/K 导航劫持文本输入 → 仅在 `e.target` 非 INPUT/TEXTAREA 时激活 J/K 导航（输入框聚焦时 J/K 不拦截，正常输入 "json"/"key" 等）。
  - H2：搜索窗口重新呼出不清空/不聚焦 → 监听 `onFocusChanged`，show 时清空查询并重新聚焦输入框（rofi 式新检索语义）。
  - H3：Ctrl+E/N 跳转时置顶检索窗口悬浮 → 跳转前先 `closeSearch()` 隐藏。
  - M2：WM Alt+F4 永久销毁 → `on_window_event(CloseRequested)` 拦截转为 hide。
  - M3：⌘/Option/Cmd+V → Ctrl+…/Alt+O/Ctrl+V（Linux/KDE 语义）。
  - 新增 Alt+O 本窗口快捷键（PRD FR-PCK-01 语义，SPEC-03 将扩展为全局注册）。
  - 最终 E2E（xdotool + 真实 Arch/X11 显示）：Alt+O 呼出搜索窗口 780×676（内容最小高度推挤，rofi 风格）；输入框获得焦点并成功键入 "hello" 触发检索；Esc 正常关闭；`wmctrl` 确认搜索窗口 hidden；无 panic/ERROR/BadMatch。`cargo test` 32 passed、`cargo clippy`、`npm run build`、`npm run tauri:build` 全绿。

## QA Result

- Status：`conditional_pass`
- Owner Back：`QA`（待人工确认 Esc 关闭行为）
- Verdict Date：2026-08-07（第三次）
- Summary：自动化全绿（fmt/32 tests/clippy/build/tauri:build）；webview 可交互（typing 生效，截图 MD5 变化）；MT1 检索→Enter→剪贴板完整链路通过（`git commit -m "QA accept"`，DB usage+1/rev+1）；无结果 Enter 不写入；搜索窗口内容渲染正确（3 条片段、输入框、快捷键提示）。
- Findings：
  - **F1（已解决）**：前次 `cargo build --release` 不嵌入前端资源→webview 不可交互。根因已定位，正确构建为 `npm run tauri:build`。
  - **F2（已解决，Orchestrator 人工确认）**：Esc 关闭搜索窗口后窗口仍可见（780×916），焦点正确切换到主窗口。`close_search_window` 的 `window.hide()` 在此环境可能无效。—— Coder 最终 E2E（xdotool + 真实 X11）已确认 Esc 正常关闭且 `wmctrl` 确认搜索窗口 hidden；2026-08-08 Orchestrator 实机人工确认 Esc 关闭搜索窗口行为正确，F2 关闭。
  - **F3（低）**：Ctrl+Enter 未触发复制（modifier 未送达 webview）；Enter 正常工作。可能是 xdotool 的 modifier 传递问题。
  - **F4（信息）**：`open_search_window` 创建新窗口（174064046）而非复用启动时的隐藏窗口（174063617）。RF2 修复的 set_size/center 作用于新窗口，有效。
  - **F5（信息）**：Ctrl+N 跨窗口跳转正常（焦点切到主窗口），但 Tab 盲驱管理表单不稳定（与 SPEC-02 同类问题）。
- Risks：
  - R1（已解除）：Esc 不隐藏搜索窗口影响体验 —— 已由人工确认 Esc 正常关闭，解除。
  - R2：Ctrl+Enter 在某些环境下 modifier 不送达（PRD FR-PCK-03 依赖此快捷键）。
- Missing Tests：
  - MT1（剪贴板 E2E）：✅ 通过。
  - MT2（Ctrl+N 表单创建）：Tab 盲驱不稳定，未完成验证。
  - MT3（Esc 关闭）：焦点切换正常但窗口未隐藏。
- Required Fixes：
  - **RF3（已验证关闭）**：修复 `close_search_window` 的 `window.hide()` 在 WebKitGTK 下不生效 —— 现行 `hide()` 实现在目标环境已验证生效（Coder E2E `wmctrl` hidden + Orchestrator 人工确认），无需 set_visible/minimize 组合改造。
  - **RF4（建议）**：验证 Ctrl+Enter 在真实用户键盘下是否正常（排除 xdotool modifier 传递问题）。
- Retest Criteria：
  - RF3 修复后：Esc 关闭搜索窗口且窗口不可见。—— ✅ 2026-08-08 Orchestrator 实机人工确认 Esc 关闭搜索窗口行为正确（且 Coder E2E 已 `wmctrl` 确认窗口 hidden）。
  - Ctrl+Enter 在真实键盘下可复制。（待真实用户验收）
  - Ctrl+N 表单创建可通过真实鼠标点击验证。（待真实用户验收）

## Orchestrator 人工验收记录（2026-08-08）

- Esc 关闭搜索窗口行为：✅ 正确（人工实机确认）。
- 关联闭环：QA F2/R1/RF3、Retest Criteria 第 3 条全部关闭。
- 待验收项（真实用户）：Ctrl+Enter 复制（RF4）、Ctrl+N 表单创建（MT2）、管理窗口 CRUD（Retest #4）。

## Retest Specific Criteria

- 1. 双窗口：Alt+O 呼出 → 搜索窗口 780×500 可见并获取焦点 → 键入查询 → Enter 复制 → 剪贴板内容正确。
- 2. Ctrl+E / Ctrl+N 跨窗口跳转正常。
- 3. Esc 关闭搜索窗口正常。
- 4. 管理窗口 CRUD：点击 +/保存按钮（真实鼠标）→ 片段写入数据库 → 重启后仍在。

## 完成定义

双窗口形态、真实数据闭环、命令薄封装与既有测试保持通过；QA `passed`；`master_plan.md` 状态一致。
