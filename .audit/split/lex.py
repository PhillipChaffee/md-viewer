"""Line-level helpers over Rust source: string-aware lexing and item chunking."""
import re

def string_start_lines(lines):
    """Return set of line indexes (0-based) whose first char lies inside a string literal or block comment."""
    inside = set()
    state = None  # None | ('str',) | ('raw', hashes) | ('block', depth)
    for idx, line in enumerate(lines):
        if state is not None:
            inside.add(idx)
        i, n = 0, len(line)
        while i < n:
            c = line[i]
            if state is None:
                if line.startswith('//', i):
                    break
                if line.startswith('/*', i):
                    state = ('block', 1); i += 2; continue
                m = re.match(r'b?r(#*)"', line[i:])
                if m and (i == 0 or not (line[i-1].isalnum() or line[i-1] == '_')):
                    state = ('raw', len(m.group(1))); i += m.end(); continue
                if c == '"':
                    state = ('str',); i += 1; continue
                if c == "'":
                    m = re.match(r"'(\\.[^']*|[^\\'])'", line[i:])
                    if m:
                        i += m.end(); continue
                i += 1
            elif state[0] == 'str':
                if c == '\\':
                    i += 2; continue
                if c == '"':
                    state = None
                i += 1
            elif state[0] == 'raw':
                end = '"' + '#' * state[1]
                j = line.find(end, i)
                if j < 0:
                    break
                state = None; i = j + len(end)
            else:
                if line.startswith('*/', i):
                    d = state[1] - 1
                    state = None if d == 0 else ('block', d); i += 2
                elif line.startswith('/*', i):
                    state = ('block', state[1] + 1); i += 2
                else:
                    i += 1
    return inside

ITEM = r'(?:pub(?:\([a-z]+\))? )?(?:unsafe )?(const|static|fn|struct|enum|impl|mod|type|trait|use)\b(.*)'

def item_name(kind, rest):
    rest = rest.strip()
    if kind == 'impl':
        return 'impl ' + rest.rstrip('{').strip()
    if kind == 'use':
        return 'use'
    m = re.match(r'([A-Za-z_]\w*)', rest)
    return m.group(1) if m else '?'

def chunks(lines, start, end, indent, instr):
    """Split lines[start:end] (0-based, end exclusive) into item chunks.
    Each chunk is (kind, name, first, last_exclusive) and includes the blank,
    comment and attribute lines that precede the item."""
    pad = ' ' * indent
    pat = re.compile('^' + pad + ITEM)
    out = []
    i = start
    chunk_start = start
    while i < end:
        line = lines[i]
        m = None if i in instr else pat.match(line)
        if not m:
            i += 1; continue
        kind, rest = m.group(1), m.group(2)
        j = i
        while True:
            l = re.sub(r'\s*//.*$', '', lines[j]).rstrip()
            at_level = (j == i) or (j not in instr and l.startswith(pad) and not l.startswith(pad + ' ') and l != '')
            if at_level and (l.endswith(';') or l.endswith('}')):
                break
            j += 1
            assert j < end, (i, line)
        out.append((kind, item_name(kind, rest), chunk_start, j + 1))
        chunk_start = j + 1
        i = j + 1
    tail = (chunk_start, end)
    return out, tail

def effective(lines):
    return sum(1 for l in lines if l.strip() and not l.strip().startswith('//'))
