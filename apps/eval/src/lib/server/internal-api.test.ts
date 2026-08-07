import { afterEach, describe, expect, it, vi } from 'vitest';
import { createInternalApi } from './internal-api.ts';

const bodyIds = (init: RequestInit | undefined) => (JSON.parse(init?.body as string) as { documentIds: string[] }).documentIds;

const stub = (make: (documentIds: string[]) => unknown[]) =>
  vi.spyOn(globalThis, 'fetch').mockImplementation((_input, init) => Promise.resolve(Response.json({ results: make(bodyIds(init)) })));

afterEach(() => vi.restoreAllMocks());

describe('createInternalApi.extract', () => {
  it('6건은 5+1로 나눠 부르고 결과를 요청 순서대로 병합한다', async () => {
    const spy = stub((ids) => ids.map((documentId) => ({ documentId, prose: `본문 ${documentId}`, title: `제목 ${documentId}` })));
    const documentIds = ['D1', 'D2', 'D3', 'D4', 'D5', 'D6'];

    const rows = await createInternalApi('https://api.test', 'tk').extract(documentIds);

    expect(spy).toHaveBeenCalledTimes(2);
    expect(spy.mock.calls.map(([, init]) => bodyIds(init))).toEqual([['D1', 'D2', 'D3', 'D4', 'D5'], ['D6']]);
    expect(spy.mock.calls[0][0]).toBe('https://api.test/internal/corpus/extract');
    expect(rows).toEqual(documentIds.map((documentId) => ({ documentId, prose: `본문 ${documentId}`, title: `제목 ${documentId}` })));
  });

  it('경계인 5건은 한 번만 부른다', async () => {
    const spy = stub((ids) => ids.map((documentId) => ({ documentId, prose: null, title: null })));

    await createInternalApi('https://api.test', 'tk').extract(['D1', 'D2', 'D3', 'D4', 'D5']);

    expect(spy).toHaveBeenCalledTimes(1);
  });

  it('title이 없는 구 배포 응답은 null로 흡수한다', async () => {
    stub((ids) => ids.map((documentId) => ({ documentId, prose: '본문' })));

    const rows = await createInternalApi('https://api.test', 'tk').extract(['D1']);

    expect(rows).toEqual([{ documentId: 'D1', prose: '본문', title: null }]);
  });
});
