import { describe, expect, it } from 'vitest';
import { fetchManuscript } from './ingest.ts';
import type { InternalApi } from './internal-api.ts';

const api = (prose: string | null, title: string | null, subtitle: string | null = null): InternalApi => ({
  extract: (ids) => Promise.resolve(ids.map((documentId) => ({ documentId, prose, title, subtitle }))),
  sendPush: () => Promise.resolve(true),
});

describe('fetchManuscript', () => {
  it('prose·title·subtitle을 돌려준다', async () => {
    await expect(fetchManuscript(api('본문입니다', '제목', '부제'), 'D0TEST01')).resolves.toEqual({
      content: '본문입니다',
      title: '제목',
      subtitle: '부제',
    });
  });

  it('prose가 없으면 사용자 문면으로 반려한다', async () => {
    await expect(fetchManuscript(api(null, null), 'D0TEST01')).resolves.toEqual({
      error: '문서를 찾을 수 없어요. 문서 ID를 확인해 주세요',
    });
  });

  it('빈 본문도 반려한다', async () => {
    await expect(fetchManuscript(api('  \n', null), 'D0TEST01')).resolves.toEqual({
      error: '문서가 비어 있어요. 내용이 있는 문서로 시도해 주세요',
    });
  });

  it('빈 문자열은 비어 있음으로 반려한다', async () => {
    await expect(fetchManuscript(api('', null), 'D0TEST01')).resolves.toEqual({
      error: '문서가 비어 있어요. 내용이 있는 문서로 시도해 주세요',
    });
  });
});
