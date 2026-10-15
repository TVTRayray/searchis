# Searchis 搜索窗口首字符丢失修复指南

适用基线：`TVTRayray/searchis`，`main` 分支提交 `a26c81afa77c6adcfd068bed60ec7ed14d3950c2`。

## 1. 问题结论

`Alt+O` 呼出搜索窗口时，Rust 侧会显示窗口并请求焦点；前端同时通过 `focus`、`visibilitychange`、Tauri `onFocusChanged` 和每 100ms 的可见性轮询判断窗口是否刚刚打开。

当前 `openAndReset()` 在这些异步事件到达时执行：

```ts
setSearchQuery("");
setError(null);
void fetchResults("");
```

窗口可能已经能够接收键盘输入，但稍晚到达的焦点或可见性事件仍会调用 `openAndReset()`。如果用户此时已经输入第一个字符，React 状态会被重新清空，于是表现为首字符或前几个字符被“吞掉”。

修复原则：

- 窗口打开时只负责聚焦、同步主题和刷新结果，不再清空查询。
- 查询应在窗口关闭时同步清空，因为关闭期间不会和用户输入竞争。
- 可见性轮询仅作为窗口被后端直接隐藏时的兜底清理。
- Escape 必须走统一的 `close()`，不能直接调用 `onClose()` 绕过清理逻辑。

## 2. 创建本地分支

```bash
git switch main
git pull --ff-only
git switch -c fix/search-first-character-loss
```

确认当前基线：

```bash
git rev-parse HEAD
```

如果输出不是上述提交，也可以继续手动修改，但应用下面的补丁时需要留意上下文是否已经变化。

## 3. 修改搜索窗口生命周期

编辑 `src/components/QuickSearchWindow.tsx`，或者在仓库根目录将以下内容保存为 `/tmp/searchis-first-character.patch` 后执行 `git apply /tmp/searchis-first-character.patch`。

```diff
diff --git a/src/components/QuickSearchWindow.tsx b/src/components/QuickSearchWindow.tsx
--- a/src/components/QuickSearchWindow.tsx
+++ b/src/components/QuickSearchWindow.tsx
@@ -56,7 +56,6 @@ export const QuickSearchWindow: React.FC<QuickSearchWindowProps> = ({
   const copyingRef = useRef(false);
   const closingRef = useRef(false);
   const isOpenRef = useRef(false);
-  const lastResetRef = useRef(0);
   const lastThemeSyncAtRef = useRef(0);
   const { setTheme } = useTheme();
 
@@ -96,6 +95,8 @@ export const QuickSearchWindow: React.FC<QuickSearchWindowProps> = ({
     closingRef.current = true;
     isOpenRef.current = false;
+    setSearchQuery("");
+    setError(null);
     onClose();
   };
 
@@ -146,14 +147,8 @@ export const QuickSearchWindow: React.FC<QuickSearchWindowProps> = ({
       void syncTheme();
     };
 
-    const openAndReset = () => {
+    const openAndRefresh = () => {
       const now = Date.now();
-      if (now - lastResetRef.current < 500) {
-        focusInput();
-        return;
-      }
-      lastResetRef.current = now;
       closingRef.current = false;
       hasBeenFocused = false;
       window.clearTimeout(focusGraceTimer);
@@ -162,7 +157,6 @@ export const QuickSearchWindow: React.FC<QuickSearchWindowProps> = ({
 
       lastThemeSyncAtRef.current = now;
       void syncTheme();
-      setSearchQuery("");
       setError(null);
       void fetchResults("");
       focusInput();
@@ -171,7 +165,7 @@ export const QuickSearchWindow: React.FC<QuickSearchWindowProps> = ({
     const activate = () => {
       if (!isOpenRef.current) {
         isOpenRef.current = true;
-        openAndReset();
+        openAndRefresh();
       } else {
         throttledSyncTheme();
         focusInput();
@@ -193,9 +187,14 @@ export const QuickSearchWindow: React.FC<QuickSearchWindowProps> = ({
       void currentWindow
         ?.isVisible()
         .then((visible) => {
           if (!visible) {
+            const wasVisible = visibleRef.current;
             visibleRef.current = false;
             isOpenRef.current = false;
+            if (wasVisible) {
+              setSearchQuery("");
+              setError(null);
+            }
             return;
           }
           if (visibleRef.current) return;
@@ -306,7 +305,7 @@ export const QuickSearchWindow: React.FC<QuickSearchWindowProps> = ({
     if (event.nativeEvent.isComposing) return;
     if (event.key === "Escape") {
       event.preventDefault();
-      onClose();
+      close();
       return;
     }
```

修改后的关键语义是：`setSearchQuery("")` 只能发生在关闭或已经隐藏的路径中，不能发生在窗口刚获得焦点的路径中。

本次不建议修改 `src-tauri/src/commands.rs` 的焦点重试。当前实现已经在 `window.is_focused()` 成功后停止 `wmctrl/xdotool` 重试；这些调用可能影响焦点到达速度，但不会直接清空 React 输入值。先消除前端确定存在的状态竞争，再观察是否仍有输入落到旧窗口的情况。

## 4. 添加回归测试

在 `e2e/create-and-search.e2e.ts` 中，找到以下代码：

```ts
await exec('xdotool', ['windowactivate', '--sync', mainWindow]);
await waitForWindow('search', false);

await setBackendTheme('dark');
```

在 `await setBackendTheme('dark');` 前插入：

```ts
// Regression: input written immediately after the picker becomes visible must
// survive delayed focus/visibility notifications.
await openSearch();
await setValue('search', inputSelectors.search, 'h');
await new Promise(resolve => setTimeout(resolve, 350));

const firstCharacter = await inWindow(
  'search',
  (_tauri, selector) => {
    const input = document.querySelector(selector as string);
    return input instanceof HTMLInputElement ? input.value : null;
  },
  inputSelectors.search,
);

if (firstCharacter !== 'h') {
  throw new Error(
    `search input lost the first character after activation: ${JSON.stringify(firstCharacter)}`,
  );
}

await closeSearch();
```

这条测试先在搜索窗口刚变为可见时写入单个字符，然后等待 350ms，使 100ms 可见性轮询、焦点通知和后台焦点重试都有机会执行，最后确认字符没有被延迟的生命周期事件清空。

注意：测试不会自动操作 KDE 全局快捷键，也不会向真实前台应用写入字符；它沿用项目现有的窗口内事件和 WebDriver 输入方式。

## 5. 格式化与验证

先执行静态构建：

```bash
npm ci
npm run build
```

然后按照项目 `AGENTS.md` 的桌面 UI 验收要求执行：

```bash
npm run test:e2e
```

最后查看改动：

```bash
git diff --check
git diff -- src/components/QuickSearchWindow.tsx e2e/create-and-search.e2e.ts
```

## 6. 手工验收

在 Arch Linux + KDE + X11 环境中至少重复 30 次：

1. 在任意文本编辑器中停留。
2. 按 `Alt+O` 呼出 Searchis。
3. 窗口出现后立即快速输入 `hello`，不要刻意等待动画或焦点稳定。
4. 确认输入框始终显示完整的 `hello`。
5. 按 Escape 关闭，再次呼出。
6. 确认新一轮搜索框为空，并继续立即输入。

还应验证：

- 点击窗口外部导致搜索窗口关闭后，再次呼出时查询为空。
- 连续打开、关闭不会保留上一次查询。
- `Ctrl+Enter`、Enter、Escape、`Ctrl+N`、`Ctrl+E` 行为没有回归。
- 输入中文时，输入法组合态不会被重置。

## 7. 如何判断是否还有第二类焦点问题

应用本补丁后：

- 如果字符曾短暂出现在搜索框中，随后消失，说明仍存在某条“打开后清空查询”的路径，应继续搜索所有 `setSearchQuery("")` 调用。
- 如果字符从未出现在搜索框中，而是输入到了原来的应用，说明是 X11/KWin 尚未完成焦点转移。这属于第二类问题，需要单独调整 Rust 侧窗口激活确认机制，而不是重新增加前端延迟清空或多次 `focus()`。

可以用下面的命令检查是否仍存在危险调用：

```bash
rg -n 'setSearchQuery\(""\)|openAndReset|lastResetRef' src
```

期望结果中，`setSearchQuery("")` 只位于关闭/隐藏清理路径；不应再出现 `openAndReset` 或 `lastResetRef`。

## 8. 提交建议

```bash
git add src/components/QuickSearchWindow.tsx e2e/create-and-search.e2e.ts
git commit -m "fix(search): preserve first input during window activation"
```

建议 PR 标题：

```text
fix(search): prevent first character loss on picker activation
```

建议 PR 说明重点：将查询重置从异步打开/聚焦阶段移动到关闭阶段，消除焦点与可见性事件晚到时覆盖用户输入的竞态，并增加延迟通知回归测试。
