// 저장되는 StablePosition은 child 키가 항상 존재해야 한다(부재 시 모바일 kotlinx 디코드가
// MissingFieldException으로 실패). wasm 경계는 Option::None을 undefined로 내보내 JSON 직렬화에서
// 키가 소실되므로 저장 전에 child: null로 복원하고, #4690 이전 binding 구형식도 함께 변환한다.

type NormalizedStablePosition = {
  chain: unknown;
  child: { dot: string; bind: 'left' | 'right' } | null;
  affinity: unknown;
};

export type StablePositionNormalization =
  { kind: 'keep' } | { kind: 'normalized'; value: NormalizedStablePosition } | { kind: 'unrecognized' };

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value);

export const normalizeStablePosition = (position: unknown): StablePositionNormalization => {
  if (!isRecord(position)) {
    return { kind: 'unrecognized' };
  }

  if ('binding' in position || 'kind' in position) {
    const { chain, binding, affinity } = position;
    if (isRecord(binding)) {
      if (binding.type === 'adjacent' && typeof binding.anchor === 'string' && (binding.bind === 'left' || binding.bind === 'right')) {
        return { kind: 'normalized', value: { chain, child: { dot: binding.anchor, bind: binding.bind }, affinity } };
      }

      if (binding.type === 'container_start') {
        return { kind: 'normalized', value: { chain, child: null, affinity } };
      }
    }

    return { kind: 'unrecognized' };
  }

  if ('child' in position) {
    return { kind: 'keep' };
  }

  const { chain, affinity } = position;
  if (Array.isArray(chain) && chain.every((dot) => typeof dot === 'string') && typeof affinity === 'string') {
    return { kind: 'normalized', value: { chain, child: null, affinity } };
  }

  // v2 position whose None child was dropped at the wasm boundary: restore the
  // child key as explicit null so kotlinx decoders (which require it) don't fail.
  if (Array.isArray(chain) && chain.every((step) => isRecord(step) && typeof step.type === 'string') && typeof affinity === 'string') {
    return { kind: 'normalized', value: { chain, child: null, affinity } };
  }

  return { kind: 'unrecognized' };
};

export type SelectionDotExtraction = { kind: 'ok'; dots: string[] } | { kind: 'unrecognized' };

const stringArray = (value: unknown): string[] | null =>
  Array.isArray(value) && value.every((v) => typeof v === 'string') ? (value as string[]) : null;

// Selections predate the current chain[]/child.dot shape by several revisions and
// unrecognized shapes are left as-is on write, so production still holds a mix of:
// current (`child` key), legacy `binding.{type,anchor}`, the untagged Rust enum
// encoding `binding.Adjacent.anchor`, and the oldest kind-tagged format where each
// chain step carries its own `{node_id, child_dot}` pair.
export const extractPositionDots = (position: unknown): SelectionDotExtraction => {
  if (!isRecord(position)) {
    return { kind: 'unrecognized' };
  }

  const { chain, child, binding, affinity, char_dot } = position;

  if ('binding' in position) {
    const chainDots = stringArray(chain);
    if (chainDots === null || !isRecord(binding)) {
      return { kind: 'unrecognized' };
    }
    if (binding.type === 'adjacent' && typeof binding.anchor === 'string') {
      return { kind: 'ok', dots: [...chainDots, binding.anchor] };
    }
    if (binding.type === 'container_start') {
      return { kind: 'ok', dots: chainDots };
    }
    if (isRecord(binding.Adjacent) && typeof binding.Adjacent.anchor === 'string') {
      return { kind: 'ok', dots: [...chainDots, binding.Adjacent.anchor] };
    }
    return { kind: 'unrecognized' };
  }

  // v2: chain of tagged ChainSegment objects. A real step references its dot; a
  // synthetic step the real owner it re-anchors through. Plus the child anchor.
  if (Array.isArray(chain) && chain.length > 0 && chain.every((step) => isRecord(step) && typeof step.type === 'string')) {
    const dots = chain.flatMap((step) => {
      const seg = step as Record<string, unknown>;
      if (seg.type === 'real' && typeof seg.dot === 'string') return [seg.dot];
      if (seg.type === 'synthetic' && typeof seg.owner === 'string') return [seg.owner];
      return [];
    });
    if (isRecord(child) && typeof child.dot === 'string') {
      dots.push(child.dot);
    }
    return { kind: 'ok', dots };
  }

  if ('child' in position) {
    const chainDots = stringArray(chain);
    if (chainDots === null) {
      return { kind: 'unrecognized' };
    }
    if (child === null) {
      return { kind: 'ok', dots: chainDots };
    }
    if (isRecord(child) && typeof child.dot === 'string') {
      return { kind: 'ok', dots: [...chainDots, child.dot] };
    }
    return { kind: 'unrecognized' };
  }

  const chainDots = stringArray(chain);
  if (chainDots !== null && typeof affinity === 'string') {
    return { kind: 'ok', dots: chainDots };
  }

  if (
    Array.isArray(chain) &&
    chain.every((step) => isRecord(step) && typeof step.node_id === 'string' && typeof step.child_dot === 'string')
  ) {
    const stepDots = (chain as Record<string, unknown>[]).flatMap((step) => [step.node_id as string, step.child_dot as string]);
    if (typeof char_dot === 'string') {
      return { kind: 'ok', dots: [...stepDots, char_dot] };
    }
    if ('kind' in position) {
      return { kind: 'ok', dots: stepDots };
    }
  }

  return { kind: 'unrecognized' };
};

export const extractSelectionDots = (selection: unknown): SelectionDotExtraction => {
  if (!isRecord(selection)) {
    return { kind: 'unrecognized' };
  }

  const anchor = extractPositionDots(selection.anchor);
  const head = extractPositionDots(selection.head);

  if (anchor.kind === 'unrecognized' || head.kind === 'unrecognized') {
    return { kind: 'unrecognized' };
  }

  return { kind: 'ok', dots: [...new Set([...anchor.dots, ...head.dots])] };
};

// The implicit-root dot string (`Dot::ROOT`), pinned for FFI clients. Kept in
// sync with `editor-crdt`'s `root_string_form_is_pinned_for_ffi_clients` test.
const ROOT_DOT = '0_AzL8n0Y58m8';

export type MigrationV1Position = {
  chain: string[];
  child: { dot: string; bind: 'left' | 'right' } | null;
  affinity: string;
};

export type MigrationV1Selection = { anchor: MigrationV1Position; head: MigrationV1Position };

const bindOf = (value: unknown): 'left' | 'right' => (typeof value === 'string' && value.toLowerCase() === 'right' ? 'right' : 'left');

// Migration-only parser: converts every storable v1 dialect (the same four the
// dot extractor reads) into the current chain[]/child DTO, preserving the
// anchor's meaning. Unlike `normalizeStablePosition` — which leaves the two
// oldest dialects `unrecognized` — this must recognize all four. A `null` return
// means the row is unmigratable, and the migration aborts on it rather than
// fabricating an offset-0 anchor.
export const normalizeStablePositionForMigration = (position: unknown): MigrationV1Position | null => {
  if (!isRecord(position)) {
    return null;
  }

  const { chain, child, binding, affinity, char_dot, kind } = position;

  if ('binding' in position) {
    const chainDots = stringArray(chain);
    if (chainDots === null || !isRecord(binding) || typeof affinity !== 'string') {
      return null;
    }
    if (binding.type === 'adjacent' && typeof binding.anchor === 'string') {
      return { chain: chainDots, child: { dot: binding.anchor, bind: bindOf(binding.bind) }, affinity };
    }
    if (binding.type === 'container_start') {
      return { chain: chainDots, child: null, affinity };
    }
    if (isRecord(binding.Adjacent) && typeof binding.Adjacent.anchor === 'string') {
      return { chain: chainDots, child: { dot: binding.Adjacent.anchor, bind: bindOf(binding.Adjacent.bind) }, affinity };
    }
    return null;
  }

  if ('child' in position) {
    const chainDots = stringArray(chain);
    if (chainDots === null || typeof affinity !== 'string') {
      return null;
    }
    if (child === null) {
      return { chain: chainDots, child: null, affinity };
    }
    if (isRecord(child) && typeof child.dot === 'string') {
      return { chain: chainDots, child: { dot: child.dot, bind: bindOf(child.bind) }, affinity };
    }
    return null;
  }

  const chainDots = stringArray(chain);
  if (chainDots !== null && typeof affinity === 'string') {
    return { chain: chainDots, child: null, affinity };
  }

  // Oldest kind-tagged dialect: each chain step carries its own {node_id, child_dot}.
  // `node_id` is a pre-CRDT ordinal (not a dot), so the block path is the child_dots
  // and the leaf anchor is char_dot. Prepend the implicit root so a fully-dead block
  // chain still degrades against root rather than failing to resolve outright.
  if (
    Array.isArray(chain) &&
    chain.length > 0 &&
    chain.every((step) => isRecord(step) && typeof step.node_id === 'string' && typeof step.child_dot === 'string') &&
    typeof affinity === 'string'
  ) {
    const blockChain = [ROOT_DOT, ...(chain as Record<string, unknown>[]).map((step) => step.child_dot as string)];
    if (kind === 'char' && typeof char_dot === 'string') {
      return { chain: blockChain, child: { dot: char_dot, bind: bindOf(position.bind) }, affinity };
    }
    if (kind === 'container_start') {
      return { chain: blockChain, child: null, affinity };
    }
    return null;
  }

  return null;
};

export const normalizeStableSelectionForMigration = (selection: unknown): MigrationV1Selection | null => {
  if (!isRecord(selection)) {
    return null;
  }
  const anchor = normalizeStablePositionForMigration(selection.anchor);
  const head = normalizeStablePositionForMigration(selection.head);
  if (anchor === null || head === null) {
    return null;
  }
  return { anchor, head };
};

// Whether a stored selection is already the v2 envelope (versioned, chain of
// tagged ChainSegment objects). The migration skips these so reruns are safe.
export const isV2Selection = (selection: unknown): boolean => {
  if (!isRecord(selection) || selection.version !== 2) {
    return false;
  }
  const isV2Position = (p: unknown): boolean =>
    isRecord(p) && Array.isArray(p.chain) && p.chain.every((step) => isRecord(step) && typeof step.type === 'string');
  return isV2Position(selection.anchor) && isV2Position(selection.head);
};

export const normalizeStableSelection = (selection: unknown): unknown => {
  if (!isRecord(selection)) {
    return selection;
  }

  const anchor = normalizeStablePosition(selection.anchor);
  const head = normalizeStablePosition(selection.head);

  if (anchor.kind === 'unrecognized' || head.kind === 'unrecognized') {
    return selection;
  }

  if (anchor.kind === 'keep' && head.kind === 'keep') {
    return selection;
  }

  return {
    ...selection,
    anchor: anchor.kind === 'normalized' ? anchor.value : selection.anchor,
    head: head.kind === 'normalized' ? head.value : selection.head,
  };
};
