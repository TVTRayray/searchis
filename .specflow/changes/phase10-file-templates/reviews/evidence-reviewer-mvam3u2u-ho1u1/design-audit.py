from pathlib import Path
import re
r=Path.cwd(); css=(r/'src/index.css').read_text()
files=list((r/'src/components').rglob('*.tsx'))+[r/'src/App.tsx']
print('COMPONENT COLOR/RADIUS AUDIT')
for pattern in [r'#[0-9a-fA-F]{3,8}\b|rgba?\(', r'rounded(?:-\[[^]]+\]|-[\w]+)?', r'borderRadius\s*:']:
    hits=[f'{p.relative_to(r)}:{i}:{line.strip()}' for p in files for i,line in enumerate(p.read_text().splitlines(),1) if re.search(pattern,line)]
    print(pattern, 'NONE' if not hits else '\n'+'\n'.join(hits))
print('LEGACY AUDIT')
for pattern in ['backdrop-filter','Figtree','HeaderBar','surface-glass','canvas-glow']:
    hits=[str(p.relative_to(r)) for p in (r/'src').rglob('*') if p.is_file() and p.suffix in ('.tsx','.ts','.css') and pattern in p.read_text()]
    print(pattern,hits)
print('FONT RESOURCES')
for rel in re.findall(r"url\('(\./assets/fonts/[^']+)'\)",css):
    print(rel, 'EXISTS' if (r/'src'/rel).is_file() else 'MISSING')
print('EXPORTED COMPONENT IMPORTS (module-edge check, not dead-code proof)')
source=list((r/'src').rglob('*.tsx'))+list((r/'src').rglob('*.ts'))
for p in files[:-1]:
    if not re.search(r'export\s+(?:const|function|class|default)',p.read_text()): continue
    importers=[]
    for q in source:
        if q==p: continue
        for rel in re.findall(r'(?:from\s+|import\s*)[\'\"]([^\'\"]+)[\'\"]',q.read_text()):
            if rel.startswith('.') and (q.parent/rel).resolve()==p.with_suffix('').resolve(): importers.append(str(q.relative_to(r)))
    print(p.relative_to(r), ', '.join(sorted(set(importers))) or 'NO STATIC IMPORT: inspect entry point')
print('CSS RADIUS DECLARATIONS NOT USING TOKENS')
for i,line in enumerate(css.splitlines(),1):
    for val in re.findall(r'border-radius:\s*([^;}]+)',line):
        if not val.startswith('var('): print(i,val)
print('TOKENS AND CONTRAST')
light=dict(re.findall(r'(--[\w-]+):\s*([^;]+);',css[:css.index('html.dark,')]))
dark=light|dict(re.findall(r'(--[\w-]+):\s*([^;]+);',css[css.index('html.dark,'):css.index('/* The manager window')]))
def rgb(s): return tuple(int(s[i:i+2],16)/255 for i in (1,3,5))
def lum(c): return sum(w*(x/12.92 if x<=.04045 else ((x+.055)/1.055)**2.4) for x,w in zip(c,(.2126,.7152,.0722)))
def contrast(a,b):
    x,y=sorted([lum(a),lum(b)]); return (y+.05)/(x+.05)
for theme,t in [('light',light),('dark',dark)]:
    print(theme)
    for key,val in t.items():
        if key.startswith(('--color-','--radius-','--font-','--motion-','--grain-opacity')): print(key,val)
    backgrounds=['--color-canvas','--color-surface','--color-surface-sidebar','--color-surface-list','--color-field']
    for fg in ['--color-fg','--color-fg-secondary','--color-fg-muted']:
        for bg in backgrounds: print(f'contrast {fg} / {bg}: {contrast(rgb(t[fg]),rgb(t[bg])):.3f}')
    for bg in ['--color-btn-primary-bg','--color-btn-primary-hover']:
        print(f'contrast primary-fg / {bg}: {contrast(rgb(t["--color-btn-primary-fg"]),rgb(t[bg])):.3f}')
    nums=[float(x.strip()) for x in re.search(r'rgba\(([^)]+)\)',t['--color-selected-bg']).group(1).split(',')]
    for bg in backgrounds[:4]:
        blended=tuple(nums[i]/255*nums[3]+x*(1-nums[3]) for i,x in enumerate(rgb(t[bg])))
        for fg in ['--color-fg','--color-fg-secondary','--color-fg-muted']:
            print(f'contrast selected {fg} / {bg}: {contrast(rgb(t[fg]),blended):.3f}')
print('HEADING / MOTION / FOCUS DECLARATIONS')
for match in re.finditer(r'([^{}]+)\{([^{}]*)\}',css):
    body=match[2]
    if any(x in body for x in ['font-family: var(--font-heading)','font-style:','outline:','animation:', 'transition:']): print(match[1].strip(),'{',body.strip(),'}')
