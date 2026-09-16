"""Lexical size estimate, NOT a Rust public reachability/coverage analyzer.
Run: rtk proxy python Docs/measure-api-surface.py
"""
from pathlib import Path
import re, json, subprocess, hashlib

root = Path(__file__).resolve().parents[1] / '../Fyrox'
names = ['fyrox', 'fyrox-impl', 'fyrox-core', 'fyrox-math', 'fyrox-graph',
         'fyrox-ui', 'fyrox-resource', 'fyrox-animation', 'fyrox-sound',
         'fyrox-material', 'fyrox-texture', 'fyrox-graphics', 'fyrox-autotile']
patterns = {
    'struct_declarations': r'^\s*pub\s+struct\s+',
    'enum_declarations': r'^\s*pub\s+enum\s+',
    'fn_declarations_including_methods': r'^\s*pub\s+(?:(?:async|unsafe|const|extern\s+"[^"]+")\s+)*fn\s+',
    'named_field_like_lines': r'^\s*pub\s+[A-Za-z_][A-Za-z_0-9]*\s*:',
}
rows = []
digest = hashlib.sha256()
for name in names:
    files = sorted((root / name / 'src').rglob('*.rs'))
    texts = []
    for path in files:
        raw = path.read_bytes()
        digest.update(path.relative_to(root).as_posix().encode())
        digest.update(raw)
        texts.append(raw.decode('utf-8'))
    source = '\n'.join(texts)
    rows.append({'crate': name, 'rs_files': len(files),
                 **{key: len(re.findall(pattern, source, re.MULTILINE))
                    for key, pattern in patterns.items()}})
report = {'method': 'lexical declaration estimate; includes cfg/test/comment/macro ambiguity; excludes dependency closure and reachability resolution',
          'engine_head': subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip(),
          'selected_source_sha256': digest.hexdigest(), 'crates': rows}
out = Path(__file__).with_name('fyrox-api-size-estimate.json')
out.write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print(json.dumps(report, ensure_ascii=False, indent=2))
