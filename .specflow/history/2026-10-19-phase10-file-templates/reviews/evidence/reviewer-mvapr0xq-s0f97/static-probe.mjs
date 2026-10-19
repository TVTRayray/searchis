import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import ts from 'typescript';

const file = 'src/components/QuickSearchWindow.tsx';
const source = fs.readFileSync(file, 'utf8');
const ast = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
let iconExpression;
function visit(node) {
  if (ts.isJsxExpression(node) && node.expression && node.expression.getText(ast).startsWith('item.pinned ?') && node.expression.getText(ast).includes('<Star')) iconExpression = node.expression.getText(ast);
  ts.forEachChild(node, visit);
}
visit(ast);
assert.ok(iconExpression, 'locate actual quick-search icon JSX');
const compiled = ts.transpileModule(`const actual = (${iconExpression});`, { compilerOptions: { jsx: ts.JsxEmit.React, target: ts.ScriptTarget.ES2020 } }).outputText;
const React = { createElement: (component, props) => ({ component, props }) };
for (const pinned of [false, true]) {
  const item = { kind: 'file', fileExt: 'xlsx', pinned };
  const actual = new Function('React', 'item', 'Star', 'FileTypeIcon', 'languageIcon', `${compiled}; return actual;`)(React, item, 'Star', 'FileTypeIcon', () => 'languageIcon');
  console.log(JSON.stringify({ case: 'xlsx quick-search type icon', pinned, expected: 'FileTypeIcon', actual: actual.component, contractSatisfied: actual.component === 'FileTypeIcon' }));
  assert.equal(actual.component, pinned ? 'Star' : 'FileTypeIcon', 'reproduce current branch behavior');
}

const css = fs.readFileSync('src/index.css', 'utf8');
const light = css.slice(css.indexOf(':root, :root[data-theme="light"]'), css.indexOf('html.dark,'));
const dark = css.slice(css.indexOf('html.dark,'), css.indexOf('/* The manager'));
const rgb = hex => hex.match(/[0-9a-f]{2}/gi).map(x => parseInt(x, 16) / 255);
const lum = c => c.map(x => x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4).reduce((s, x, i) => s + x * [0.2126, 0.7152, 0.0722][i], 0);
const ratio = (a, b) => (Math.max(lum(a), lum(b)) + 0.05) / (Math.min(lum(a), lum(b)) + 0.05);
for (const [theme, block] of [['light', light], ['dark', dark]]) {
  const value = name => block.match(new RegExp(`--${name}:\\s*([^;]+);`))[1];
  const surface = rgb(value('color-surface'));
  const fg = rgb(value('color-fg'));
  const selected = value('color-selected-bg').match(/[\d.]+/g).map(Number);
  const selectedBg = surface.map((c, i) => selected[i] / 255 * selected[3] + c * (1 - selected[3]));
  const values = {
    body: ratio(fg, surface), muted: ratio(rgb(value('color-fg-muted')), surface),
    selected: ratio(fg, selectedBg), primary: ratio(rgb(value('color-btn-primary-fg')), rgb(value('color-btn-primary-bg'))),
    notice: ratio(rgb(value('color-fg-secondary')), selectedBg),
  };
  console.log(theme, JSON.stringify(Object.fromEntries(Object.entries(values).map(([k,v]) => [k, +v.toFixed(2)]))));
  assert.ok(Object.values(values).every(v => v >= 4.5));
}
function walk(dir) { return fs.readdirSync(dir, {withFileTypes:true}).flatMap(e => e.isDirectory() ? walk(path.join(dir,e.name)) : /\.tsx?$/.test(e.name) ? [path.join(dir,e.name)] : []); }
const files = walk('src');
const imported = new Set();
for (const f of files) {
  const a = ts.createSourceFile(f, fs.readFileSync(f,'utf8'), ts.ScriptTarget.Latest, true);
  for (const n of a.statements) if (ts.isImportDeclaration(n) && ts.isStringLiteral(n.moduleSpecifier) && n.moduleSpecifier.text.startsWith('.')) {
    const base = path.resolve(path.dirname(f), n.moduleSpecifier.text);
    for (const candidate of [base,base+'.tsx',base+'.ts',path.join(base,'index.tsx')]) if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) imported.add(candidate);
  }
}
const unimported = files.filter(f => f.startsWith('src/components/') && f.endsWith('.tsx') && !imported.has(path.resolve(f)));
console.log('Component modules without static imports:', JSON.stringify(unimported));
console.log('HeaderBar references:', files.filter(f => fs.readFileSync(f,'utf8').includes('HeaderBar')));
console.log('AboutModal remains imported (SPEC-24 deletion deferred):', imported.has(path.resolve('src/components/AboutModal.tsx')));
console.log('Static review probe finished; pinned-file icon contract violation reproduced, not an application-suite pass.');
