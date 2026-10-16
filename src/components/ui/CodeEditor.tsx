import { useEffect, useRef } from 'react';
import { Compartment, EditorState, Extension, Transaction } from '@codemirror/state';
import { EditorView, keymap, placeholder as placeholderExtension } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { HighlightStyle, StreamLanguage, syntaxHighlighting } from '@codemirror/language';
import { tags } from '@lezer/highlight';
import type { SnippetLanguage } from '../../types/snippet';

interface CodeEditorProps {
  value: string;
  language: SnippetLanguage;
  onChange: (value: string) => void;
  placeholder?: string;
  ariaLabel?: string;
  className?: string;
}

// 语法高亮只引用 SPEC-20 声明的 token，明暗主题随 CSS 变量切换。
const highlightStyle = HighlightStyle.define([
  { tag: [tags.keyword, tags.operatorKeyword, tags.modifier, tags.definitionKeyword], color: 'var(--color-syntax-keyword)' },
  { tag: [tags.string, tags.special(tags.string), tags.regexp], color: 'var(--color-syntax-string)' },
  { tag: [tags.comment, tags.lineComment, tags.blockComment], color: 'var(--color-syntax-comment)', fontStyle: 'normal' },
  { tag: [tags.number, tags.bool, tags.null, tags.atom], color: 'var(--color-syntax-number)' },
  { tag: [tags.propertyName, tags.attributeName, tags.labelName], color: 'var(--color-syntax-number)' },
  { tag: [tags.heading, tags.typeName, tags.className], color: 'var(--color-syntax-keyword)' },
]);

const editorTheme = EditorView.theme({
  '&': { backgroundColor: 'transparent', color: 'var(--color-fg)', fontSize: '13.5px' },
  '&.cm-focused': { outline: 'none' },
  '.cm-scroller': { fontFamily: 'var(--font-mono)', lineHeight: '1.6', minHeight: '176px', maxHeight: '420px' },
  '.cm-content': { padding: '2px 0', caretColor: 'var(--color-accent)' },
  '.cm-line': { padding: '0 2px' },
  '.cm-cursor': { borderLeftColor: 'var(--color-accent)' },
  '.cm-selectionBackground, &.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, ::selection': {
    backgroundColor: 'var(--color-selected-bg)',
  },
  '.cm-placeholder': { color: 'var(--color-fg-secondary)' },
});

/** 语言包按需加载：只有切到该语言时才会拉取对应 chunk。 */
async function loadLanguage(language: SnippetLanguage): Promise<Extension> {
  switch (language) {
    case 'bash': {
      const { shell } = await import('@codemirror/legacy-modes/mode/shell');
      return StreamLanguage.define(shell);
    }
    case 'sql': {
      const { sql } = await import('@codemirror/lang-sql');
      return sql();
    }
    case 'yaml': {
      const { yaml } = await import('@codemirror/lang-yaml');
      return yaml();
    }
    case 'json': {
      const { json } = await import('@codemirror/lang-json');
      return json();
    }
    case 'toml': {
      const { toml } = await import('@codemirror/legacy-modes/mode/toml');
      return StreamLanguage.define(toml);
    }
    case 'ini': {
      const { properties } = await import('@codemirror/legacy-modes/mode/properties');
      return StreamLanguage.define(properties);
    }
    default:
      return [];
  }
}

/**
 * 按初始正文选择换行符，使 `sliceDoc()` 往返不改变原始字节：
 * CodeMirror 默认把 \r、\r\n 都当作换行并在 toString 时统一成 \n。
 */
function detectLineSeparator(value: string): string | null {
  if (value.includes('\r\n')) return '\r\n';
  if (value.includes('\r')) return '\r';
  return null;
}

/**
 * CodeMirror 6 编辑区。受控值：外部 value 与文档不同才回写，避免打断 IME 组合与撤销栈。
 * 调用方必须在切换片段时通过 `key` 重新挂载，并保证挂载时 `value` 已是目标正文；
 * 外部回写不进入撤销历史，因此 Ctrl+Z 不会带回其他片段的正文。
 */
export function CodeEditor({ value, language, onChange, placeholder, ariaLabel, className }: CodeEditorProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const languageCompartment = useRef(new Compartment());
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;
  const initialValue = useRef(value);
  const initialLanguage = useRef(language);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return undefined;
    const lineSeparator = detectLineSeparator(initialValue.current);
    const view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: initialValue.current,
        extensions: [
          ...(lineSeparator ? [EditorState.lineSeparator.of(lineSeparator)] : []),
          history(),
          keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
          syntaxHighlighting(highlightStyle),
          editorTheme,
          languageCompartment.current.of([]),
          EditorView.lineWrapping,
          EditorView.contentAttributes.of({
            'aria-label': ariaLabel ?? '文本内容',
            'data-gramm': 'false',
          }),
          ...(placeholder ? [placeholderExtension(placeholder)] : []),
          EditorView.updateListener.of(update => {
            if (update.docChanged) onChangeRef.current(update.state.sliceDoc());
          }),
        ],
      }),
    });
    viewRef.current = view;
    let cancelled = false;
    void loadLanguage(initialLanguage.current).then(extension => {
      if (!cancelled) view.dispatch({ effects: languageCompartment.current.reconfigure(extension) });
    });
    return () => {
      cancelled = true;
      view.destroy();
      viewRef.current = null;
    };
    // 编辑器实例只在挂载时创建；value / language 变化由下方 effect 增量同步。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return undefined;
    let cancelled = false;
    void loadLanguage(language).then(extension => {
      if (!cancelled) view.dispatch({ effects: languageCompartment.current.reconfigure(extension) });
    });
    return () => {
      cancelled = true;
    };
  }, [language]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    const current = view.state.sliceDoc();
    if (current !== value) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: value },
        annotations: Transaction.addToHistory.of(false),
      });
    }
  }, [value]);

  return <div ref={hostRef} className={className} data-language={language} />;
}
