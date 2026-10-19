# UX Design: phase10-file-templates

> 产品设计工件。视觉源：docs/prds/file-templates-ui-requirements.md 第二部分；定版预览 docs/prds/file-templates-ui-theme-preview.html；项目宪章 docs/design/design-charter.md。

## Design Impact

- 档位：design_system
- 关联宪章条目：设计原则 1～5、签名元素全部、反模式全部（docs/design/design-charter.md）；token 表（docs/design/design-tokens.md）。

## 视觉意图

- 本次变更要达成的视觉/体验目标（一句话）：温暖、不透明、带纸张质感的工具界面，键盘取用依旧最快，文本片段与文件模板一眼可分。
- 与现状的差异：去掉液态玻璃的半透明面板与氛围光；浅色由电光蓝改为森林绿加鼠尾草绿，深色改为暖炭加陶土色；UI 字体由 Figtree 改为 Source Sans 3，标题新增 Playfair Display；HeaderBar 删除，按钮移到左栏底部；设置由弹窗改为视图。

## 受影响表面

- 窗口/组件/状态列表：
  - 快搜窗口：结果行 Q-A（类型图标、标题、副标题 Key · 分组 · 使用次数、徽标、Ctrl+N 序号、匹配高亮）、底栏提示随类型切换。
  - 管理窗口左栏：全部/置顶/最近使用、分组树（折叠、加号、未分组、计数）、标签区（折叠）、回收站、底部按钮栏。
  - 管理窗口中栏：过滤框加“+”菜单、类型筛选（全部/文本/文件）、混排列表、拖入导入。
  - 管理窗口右栏：文本片段表单增加分组与语言、CodeMirror 高亮；文件模板元数据表单、文件信息卡片、打开/打开方式/导出/带格式复制按钮、右上角放入回收站。
  - 设置视图 S1：分节导航、文件与导出、关于分节。
  - 快捷键速查：按文本片段与文件模板分组。
- 是否动 tokens（design_system 档必填）：是。全部颜色 token 替换为植物/暖炭两套；新增 color-highlight、icon-bg、btn-secondary-border、focus-ring、文件类型四色、语法高亮四色、radius-window/card/row/pill、font-heading/body/mono、grain-opacity；删除 surface-glass-header、surface-glass-sidebar、canvas-glow 系列、blur-radius。原因：D-17 废弃玻璃并定版新视觉，后续 spec 需要文件类型与语法高亮 token。

## Before / After

- 参考 mockup 路径 / 截图路径（必填，实施前必须存在）：After 为 docs/prds/file-templates-ui-theme-preview.html（浏览器直接打开）；Before 为 Phase 9 截图 artifacts/spec-19（若本地存在）或当前 main 分支实机。
- 关键状态（loading/empty/error/选中/敏感）是否覆盖：预览覆盖选中、悬停、空状态与文件/文本混排；loading（扫描中）、error（错误码提示）、敏感遮挡与文件缺失标记由各 spec 实现时按宪章补齐，QA 按清单检查。

## 与宪章的符合性

- 不透明表面、无 backdrop-filter：符合原则“纸面而非玻璃”。
- 键盘优先：快搜动效 150ms 与 200ms 上限，管理窗口 300ms，列表无卡片上浮：符合原则“速度优先于装饰”。
- 两套主题强调色不同（鼠尾草绿与陶土色）：宪章明确接受（D-17），以赤陶色作为浅色主按钮悬停色衔接。
- 中文不使用斜体、标题衬线只用于 18px 及以上：符合排版原则。
- 无违反项，无需额外审批。

## 验收提示（给 QA）

- 本变更在 visual-qa-checklist.md 中应重点检查的条目：Searchis 专项全部条目（无 backdrop-filter、token 逐项对照、HeaderBar 零引用、字体真实加载、颗粒层不拦截点击、Q-A 结果行、徽标规则、文件模板不显示敏感开关）；主题与对比度；动效与 reduced-motion；状态完整性（扫描中、文件缺失、导出路径提示）。
- 每个 spec 只检查本 spec 触及的表面，但 visual-foundation 之后的每个 spec 都要回归“无硬编码颜色”与“明暗双模截图”。
