import { describe, expect, it } from 'vitest';
import { resolveLinks } from './persist.ts';

describe('resolveLinks', () => {
  it('인덱스를 같은 배치의 항목 id로 바꾼다', () => {
    const ids = ['a', 'b', 'c'];
    expect(resolveLinks(ids, 2, [0, 1])).toEqual([
      { itemId: 'c', targetItemId: 'a', ord: 0 },
      { itemId: 'c', targetItemId: 'b', ord: 1 },
    ]);
  });

  it('범위 밖 인덱스는 버린다', () => {
    expect(resolveLinks(['a', 'b'], 1, [0, 5, -1])).toEqual([{ itemId: 'b', targetItemId: 'a', ord: 0 }]);
  });

  it('자기 자신을 가리키는 연결은 버린다', () => {
    expect(resolveLinks(['a', 'b'], 1, [1, 0])).toEqual([{ itemId: 'b', targetItemId: 'a', ord: 0 }]);
  });

  it('중복 대상은 한 번만 남긴다', () => {
    expect(resolveLinks(['a', 'b'], 1, [0, 0])).toEqual([{ itemId: 'b', targetItemId: 'a', ord: 0 }]);
  });

  it('정수가 아닌 인덱스는 버린다', () => {
    expect(resolveLinks(['a', 'b'], 1, [0.5, NaN, 0])).toEqual([{ itemId: 'b', targetItemId: 'a', ord: 0 }]);
  });
});
