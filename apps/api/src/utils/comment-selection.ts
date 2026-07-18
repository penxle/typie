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
