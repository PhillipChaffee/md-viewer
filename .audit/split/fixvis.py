#!/usr/bin/env python3
"""Run cargo check, add every privately-accessed field/method to vis.json. Prints remaining other errors."""
import json, pathlib, re, subprocess
HERE = pathlib.Path(__file__).parent
vis = json.loads((HERE/'vis.json').read_text()) if (HERE/'vis.json').exists() else {'fields': [], 'methods': []}
fields, methods, types = set(vis['fields']), set(vis['methods']), set(vis.get('types', []))
p = subprocess.run(['cargo','check','--tests','--message-format=json'], cwd='/home/user/md-viewer', capture_output=True, text=True)
other = []
n = 0
for line in p.stdout.splitlines():
    try: d = json.loads(line)
    except Exception: continue
    m = d.get('message')
    if not m or m.get('level') not in ('error', 'warning'): continue
    code = (m.get('code') or {}).get('code')
    msg = m['message']
    if code in ('E0616', 'E0451'):
        mm = re.search(r"field `(\w+)` of (?:struct|type) `(?:[\w:]*::)?(\w+)(?:<.*>)?` is private", msg)
        if mm: fields.add(f'{mm.group(2)}.{mm.group(1)}'); n += 1; continue
    if code == 'E0451':
        mm = re.search(r"fields? (.*) of struct `(?:[\w:]*::)?(\w+)", msg)
        if mm:
            for fld in re.findall(r'`(\w+)`', mm.group(1)): fields.add(f'{mm.group(2)}.{fld}')
            n += 1; continue
    mm = re.match(r"type `(?:[\w:]*::)?(\w+)` is (?:private|more private than)", msg)
    if mm: types.add(mm.group(1)); n += 1; continue
    if code == 'E0624':
        done = False
        for sp in m['spans']:
            if sp.get('label') and 'defined here' in sp['label']:
                name = re.search(r'fn (\w+)', sp['text'][0]['text']).group(1)
                methods.add(sp['file_name'][4:] + ':' + name); done = True
        if done: n += 1; continue
    other.append(m['rendered'])
(HERE/'vis.json').write_text(json.dumps({'fields': sorted(fields), 'methods': sorted(methods), 'types': sorted(types)}, indent=1))
print(f'vis additions handled: {n}; other diagnostics: {len(other)}')
import sys
for o in other[: int(sys.argv[1]) if len(sys.argv) > 1 else 40]:
    print(o)
