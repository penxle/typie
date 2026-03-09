// spell-checker:words ENDOFCHAIN FREESECT FATSECT DIFAT NOSTREAM CLSID

const SECTOR_SIZE = 512;
const MINI_SECTOR_SIZE = 64;
const MINI_CUTOFF = 4096;
const ENDOFCHAIN = 0xff_ff_ff_fe;
const FREESECT = 0xff_ff_ff_ff;
const FATSECT = 0xff_ff_ff_fd;
const NOSTREAM = 0xff_ff_ff_ff;

type Node = {
  name: string;
  type: 0x01 | 0x02 | 0x05;
  data: Uint8Array | null;
  children: Node[];
  id: number;
  startSect: number;
  size: number;
  leftId: number;
  rightId: number;
  childId: number;
};

function makeNode(name: string, type: 0x01 | 0x02 | 0x05, data: Uint8Array | null): Node {
  return {
    name,
    type,
    data,
    children: [],
    id: -1,
    startSect: ENDOFCHAIN,
    size: data?.length ?? 0,
    leftId: NOSTREAM,
    rightId: NOSTREAM,
    childId: NOSTREAM,
  };
}

/** Minimal OLE2/CFB writer (MS-CFB v3, 512-byte sectors). */
export function buildOle2(streams: { path: string; data: Uint8Array }[]): Uint8Array {
  // --- directory tree ---
  const root = makeNode('Root Entry', 0x05, null);

  for (const { path, data } of streams) {
    const parts = path.split('/');
    let parent = root;
    for (let i = 0; i < parts.length; i++) {
      if (i < parts.length - 1) {
        let child = parent.children.find((c) => c.name === parts[i] && c.type === 0x01);
        if (!child) {
          child = makeNode(parts[i], 0x01, null);
          parent.children.push(child);
        }
        parent = child;
      } else {
        parent.children.push(makeNode(parts[i], 0x02, data));
      }
    }
  }

  // --- flatten + assign IDs (pre-order) ---
  const entries: Node[] = [];
  const walk = (node: Node) => {
    node.id = entries.length;
    entries.push(node);
    node.children.sort(cmpNode);
    for (const c of node.children) walk(c);
  };
  walk(root);

  // --- BST pointers per storage ---
  for (const e of entries) {
    if (e.children.length > 0) e.childId = buildBst(e.children);
  }

  // --- classify streams ---
  const mini: Node[] = [];
  const regular: Node[] = [];
  for (const e of entries) {
    if (e.type !== 0x02 || !e.data || e.data.length === 0) continue;
    (e.data.length < MINI_CUTOFF ? mini : regular).push(e);
  }

  // --- mini stream + mini FAT ---
  const miniFat: number[] = [];
  const miniParts: Uint8Array[] = [];

  for (const node of mini) {
    const d = node.data ?? new Uint8Array(0);
    const cnt = Math.ceil(d.length / MINI_SECTOR_SIZE);
    node.startSect = miniFat.length;
    for (let i = 0; i < cnt; i++) {
      miniFat.push(i < cnt - 1 ? miniFat.length + 1 : ENDOFCHAIN);
    }
    const padded = new Uint8Array(cnt * MINI_SECTOR_SIZE);
    padded.set(d);
    miniParts.push(padded);
  }

  const miniStreamData = concatBytes(miniParts);

  // --- sector counts ---
  const dirSects = Math.ceil(entries.length / 4);
  const mfSects = miniFat.length > 0 ? Math.ceil((miniFat.length * 4) / SECTOR_SIZE) : 0;
  const msSects = miniStreamData.length > 0 ? Math.ceil(miniStreamData.length / SECTOR_SIZE) : 0;
  const regCounts = regular.map((n) => Math.ceil((n.data?.length ?? 0) / SECTOR_SIZE));
  const regTotal = regCounts.reduce((a, b) => a + b, 0);

  let fatSects = 1;
  for (;;) {
    const total = fatSects + dirSects + mfSects + msSects + regTotal;
    if (Math.ceil(total / 128) <= fatSects) break;
    fatSects++;
  }
  if (fatSects > 109) throw new Error('File too large for single DIFAT');

  // --- sector layout: FAT | Dir | MiniFAT | MiniStream | Regular ---
  let s = 0;
  const fatStart = s;
  s += fatSects;
  const dirStart = s;
  s += dirSects;
  const mfStart = s;
  s += mfSects;
  const msStart = s;
  s += msSects;

  for (const [i, element] of regular.entries()) {
    element.startSect = s;
    s += regCounts[i];
  }

  if (msSects > 0) {
    root.startSect = msStart;
    root.size = miniStreamData.length;
  }

  const totalSects = s;

  // --- FAT ---
  const fat = new Uint32Array(fatSects * 128).fill(FREESECT);
  for (let i = 0; i < fatSects; i++) fat[fatStart + i] = FATSECT;
  chainFat(fat, dirStart, dirSects);
  chainFat(fat, mfStart, mfSects);
  chainFat(fat, msStart, msSects);
  for (let i = 0; i < regular.length; i++) chainFat(fat, regular[i].startSect, regCounts[i]);

  // --- assemble ---
  const buf = new Uint8Array(512 + totalSects * SECTOR_SIZE);
  const view = new DataView(buf.buffer);

  // header
  writeHeader(view, fatSects, fatStart, dirStart, mfSects > 0 ? mfStart : ENDOFCHAIN, mfSects);

  // FAT sectors
  for (let i = 0; i < fatSects; i++) {
    const off = sectOff(fatStart + i);
    for (let j = 0; j < 128; j++) view.setUint32(off + j * 4, fat[i * 128 + j], true);
  }

  // directory entries
  for (let i = 0; i < entries.length; i++) writeDirEntry(view, sectOff(dirStart) + i * 128, entries[i]);

  // mini FAT
  if (mfSects > 0) {
    const off = sectOff(mfStart);
    for (let i = 0; i < mfSects * 128; i++) view.setUint32(off + i * 4, FREESECT, true);
    for (let i = 0; i < miniFat.length; i++) view.setUint32(off + i * 4, miniFat[i], true);
  }

  // mini stream
  if (msSects > 0) buf.set(miniStreamData, sectOff(msStart));

  // regular streams
  for (const n of regular) {
    if (n.data) buf.set(n.data, sectOff(n.startSect));
  }

  return buf;
}

function sectOff(n: number): number {
  return (n + 1) * SECTOR_SIZE;
}

function chainFat(fat: Uint32Array, start: number, count: number): void {
  for (let i = 0; i < count; i++) fat[start + i] = i < count - 1 ? start + i + 1 : ENDOFCHAIN;
}

/** MS-CFB §2.6.1: shorter name < longer name, then case-insensitive lexicographic */
function cmpNode(a: Node, b: Node): number {
  const au = a.name.toUpperCase();
  const bu = b.name.toUpperCase();
  if (au.length !== bu.length) return au.length - bu.length;
  return au < bu ? -1 : au > bu ? 1 : 0;
}

/** Build balanced BST from sorted children, returns root node ID */
function buildBst(sorted: Node[]): number {
  if (sorted.length === 0) return NOSTREAM;
  const mid = sorted.length >> 1;
  sorted[mid].leftId = buildBst(sorted.slice(0, mid));
  sorted[mid].rightId = buildBst(sorted.slice(mid + 1));
  return sorted[mid].id;
}

function writeHeader(view: DataView, fatSects: number, fatStart: number, dirStart: number, mfStart: number, mfSects: number): void {
  const sig = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
  for (let i = 0; i < 8; i++) view.setUint8(i, sig[i]);

  view.setUint16(0x18, 0x00_3e, true); // minor version
  view.setUint16(0x1a, 0x00_03, true); // major version (v3)
  view.setUint16(0x1c, 0xff_fe, true); // byte order (LE)
  view.setUint16(0x1e, 0x00_09, true); // sector shift (2^9 = 512)
  view.setUint16(0x20, 0x00_06, true); // mini sector shift (2^6 = 64)
  // 0x22: 6 bytes reserved (zero)
  // 0x28: number of directory sectors (must be 0 for v3)
  view.setUint32(0x2c, fatSects, true);
  view.setUint32(0x30, dirStart, true);
  // 0x34: transaction signature (zero)
  view.setUint32(0x38, MINI_CUTOFF, true);
  view.setUint32(0x3c, mfStart, true);
  view.setUint32(0x40, mfSects, true);
  view.setUint32(0x44, ENDOFCHAIN, true); // first DIFAT sector (none)
  // 0x48: number of DIFAT sectors (zero)

  // DIFAT array (109 entries at 0x4C)
  for (let i = 0; i < 109; i++) {
    view.setUint32(0x4c + i * 4, i < fatSects ? fatStart + i : FREESECT, true);
  }
}

function writeDirEntry(view: DataView, off: number, entry: Node): void {
  const len = Math.min(entry.name.length, 31);
  for (let i = 0; i < len; i++) view.setUint16(off + i * 2, entry.name.codePointAt(i) ?? 0, true);

  view.setUint16(off + 0x40, (len + 1) * 2, true); // name byte length (incl. null)
  view.setUint8(off + 0x42, entry.type);
  view.setUint8(off + 0x43, 0x01); // color: black
  view.setUint32(off + 0x44, entry.leftId, true);
  view.setUint32(off + 0x48, entry.rightId, true);
  view.setUint32(off + 0x4c, entry.childId, true);
  // 0x50: CLSID (16 bytes zero), 0x60: state bits (zero), 0x64/0x6C: timestamps (zero)
  view.setUint32(off + 0x74, entry.startSect, true);
  view.setUint32(off + 0x78, entry.size, true);
}

function concatBytes(parts: Uint8Array[]): Uint8Array {
  const total = parts.reduce((a, b) => a + b.length, 0);
  const result = new Uint8Array(total);
  let off = 0;
  for (const p of parts) {
    result.set(p, off);
    off += p.length;
  }
  return result;
}
