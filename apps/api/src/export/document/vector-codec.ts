import type { VectorOp, VectorPage, VectorPathCommand } from './vector';

const BINARY_MAGIC = 0x31_56_45_54; // TVE1

const OP_FILL_PATH = 0;
const OP_STROKE_PATH = 1;

// const FILL_RULE_WINDING = 0;
const FILL_RULE_EVEN_ODD = 1;

// const LINE_CAP_BUTT = 0;
const LINE_CAP_ROUND = 1;
const LINE_CAP_SQUARE = 2;

// const LINE_JOIN_MITER = 0;
const LINE_JOIN_ROUND = 1;
const LINE_JOIN_BEVEL = 2;

const CMD_MOVE_TO = 0;
const CMD_LINE_TO = 1;
const CMD_QUAD_TO = 2;
const CMD_CUBIC_TO = 3;
const CMD_CLOSE_PATH = 4;

const parsePath = (view: DataView, offsetRef: { value: number }, count: number): VectorPathCommand[] => {
  const path: VectorPathCommand[] = [];

  for (let i = 0; i < count; i++) {
    const cmd = view.getUint8(offsetRef.value);
    offsetRef.value += 1;

    if (cmd === CMD_MOVE_TO) {
      const x = view.getFloat32(offsetRef.value, true);
      const y = view.getFloat32(offsetRef.value + 4, true);
      offsetRef.value += 8;
      path.push({ type: 'moveTo', x, y });
      continue;
    }

    if (cmd === CMD_LINE_TO) {
      const x = view.getFloat32(offsetRef.value, true);
      const y = view.getFloat32(offsetRef.value + 4, true);
      offsetRef.value += 8;
      path.push({ type: 'lineTo', x, y });
      continue;
    }

    if (cmd === CMD_QUAD_TO) {
      const cx = view.getFloat32(offsetRef.value, true);
      const cy = view.getFloat32(offsetRef.value + 4, true);
      const x = view.getFloat32(offsetRef.value + 8, true);
      const y = view.getFloat32(offsetRef.value + 12, true);
      offsetRef.value += 16;
      path.push({ type: 'quadTo', cx, cy, x, y });
      continue;
    }

    if (cmd === CMD_CUBIC_TO) {
      const c1x = view.getFloat32(offsetRef.value, true);
      const c1y = view.getFloat32(offsetRef.value + 4, true);
      const c2x = view.getFloat32(offsetRef.value + 8, true);
      const c2y = view.getFloat32(offsetRef.value + 12, true);
      const x = view.getFloat32(offsetRef.value + 16, true);
      const y = view.getFloat32(offsetRef.value + 20, true);
      offsetRef.value += 24;
      path.push({ type: 'cubicTo', c1x, c1y, c2x, c2y, x, y });
      continue;
    }

    if (cmd === CMD_CLOSE_PATH) {
      path.push({ type: 'closePath' });
      continue;
    }

    throw new Error(`Unknown vector path command: ${cmd}`);
  }

  return path;
};

export const parseVectorPageBinary = (bytes: Uint8Array): Omit<VectorPage, 'externalElements'> => {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const offset = { value: 0 };

  const magic = view.getUint32(offset.value, true);
  offset.value += 4;
  if (magic !== BINARY_MAGIC) {
    throw new Error(`Unexpected vector binary magic: ${magic.toString(16)}`);
  }

  const width = view.getFloat32(offset.value, true);
  const height = view.getFloat32(offset.value + 4, true);
  offset.value += 8;

  const opCount = view.getUint32(offset.value, true);
  offset.value += 4;

  const ops: VectorOp[] = [];

  for (let i = 0; i < opCount; i++) {
    const opTag = view.getUint8(offset.value);
    offset.value += 1;

    const pathCount = view.getUint32(offset.value, true);
    offset.value += 4;
    const path = parsePath(view, offset, pathCount);

    const color: [number, number, number, number] = [
      view.getUint8(offset.value),
      view.getUint8(offset.value + 1),
      view.getUint8(offset.value + 2),
      view.getUint8(offset.value + 3),
    ];
    offset.value += 4;

    if (opTag === OP_FILL_PATH) {
      const fillRuleTag = view.getUint8(offset.value);
      offset.value += 1;
      ops.push({
        type: 'fillPath',
        path,
        color,
        fillRule: fillRuleTag === FILL_RULE_EVEN_ODD ? 'evenOdd' : 'winding',
      });
      continue;
    }

    if (opTag === OP_STROKE_PATH) {
      const width = view.getFloat32(offset.value, true);
      offset.value += 4;

      const lineCapTag = view.getUint8(offset.value);
      const lineJoinTag = view.getUint8(offset.value + 1);
      offset.value += 2;

      const lineCap = lineCapTag === LINE_CAP_ROUND ? 'round' : lineCapTag === LINE_CAP_SQUARE ? 'square' : 'butt';
      const lineJoin = lineJoinTag === LINE_JOIN_ROUND ? 'round' : lineJoinTag === LINE_JOIN_BEVEL ? 'bevel' : 'miter';

      ops.push({
        type: 'strokePath',
        path,
        color,
        width,
        lineCap,
        lineJoin,
      });
      continue;
    }

    throw new Error(`Unknown vector op tag: ${opTag}`);
  }

  return { width, height, ops };
};
