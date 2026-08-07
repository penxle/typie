import type { InternalApi } from './internal-api.ts';

export const fetchManuscript = async (
  api: InternalApi,
  refId: string,
): Promise<{ content: string; title: string | null } | { error: string }> => {
  const [row] = await api.extract([refId]);
  if (!row || row.prose == null) return { error: '문서를 찾을 수 없어요. 문서 ID를 확인해 주세요' };
  if (row.prose.trim().length === 0) return { error: '문서가 비어 있어요. 내용이 있는 문서로 시도해 주세요' };
  return { content: row.prose, title: row.title ?? null };
};
