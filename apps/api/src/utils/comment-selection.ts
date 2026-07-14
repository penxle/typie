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
