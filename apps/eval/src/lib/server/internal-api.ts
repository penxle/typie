export type InternalApi = {
  // 실서비스에서 원고 본문을 가져오는 유일한 경로. 코퍼스에 저장되는 본문은 이 프로즈다 —
  // document_states의 평문과 달라서, 다른 경로를 쓰면 세대가 코퍼스와 다른 형태의 글을 읽는다.
  extract: (
    documentIds: string[],
  ) => Promise<{ documentId: string; prose: string | null; title: string | null; subtitle: string | null }[]>;
  sendPush: (documentId: string, title: string, body: string) => Promise<boolean>;
};

// extract는 요청당 5건이 상한이다.
const EXTRACT_BATCH = 5;

const chunk = <T>(items: T[], size: number): T[][] => {
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += size) {
    out.push(items.slice(i, i + size));
  }
  return out;
};

export const createInternalApi = (base: string, key: string): InternalApi => {
  const headers = { authorization: `Bearer ${key}` };

  return {
    extract: async (documentIds) => {
      const results: { documentId: string; prose: string | null; title: string | null; subtitle: string | null }[] = [];
      for (const batch of chunk(documentIds, EXTRACT_BATCH)) {
        const response = await fetch(`${base}/internal/corpus/extract`, {
          method: 'POST',
          headers: { ...headers, 'content-type': 'application/json' },
          body: JSON.stringify({ documentIds: batch }),
        });
        if (!response.ok) {
          throw new Error(`corpus extract failed: ${response.status}`);
        }
        // title·subtitle은 뒤에 붙은 필드다 — api가 아직 구 배포면 없는 채로 온다.
        const body = (await response.json()) as {
          results: { documentId: string; prose: string | null; title?: string | null; subtitle?: string | null }[];
        };
        results.push(
          ...body.results.map((row) => ({
            documentId: row.documentId,
            prose: row.prose,
            title: row.title ?? null,
            subtitle: row.subtitle ?? null,
          })),
        );
      }
      return results;
    },
    sendPush: async (documentId, title, body) => {
      const response = await fetch(`${base}/internal/push`, {
        method: 'POST',
        headers: { ...headers, 'content-type': 'application/json' },
        body: JSON.stringify({ documentId, title, body }),
      });
      return response.ok;
    },
  };
};
