// 원고 접근 도구의 실행기 — 스펙 §4. 원고는 프롬프트 접두부가 아니라 이 두 도구로 접근한다.

export const READ_CAP = 4000;

export type ReadResult = { start: number; end: number; text: string; truncated: boolean };

export const executeRead = (content: string, start: number, end: number): ReadResult => {
  const s = Math.max(0, Math.min(Math.floor(start), content.length));
  const requested = Math.max(s, Math.min(Math.floor(end), content.length));
  const truncated = requested - s > READ_CAP;
  const e = truncated ? s + READ_CAP : requested;
  return { start: s, end: e, text: content.slice(s, e), truncated };
};

const GREP_MAX = 50;
const CONTEXT = 40;

export type GrepMatch = { start: number; end: number; text: string; context: string };
export type GrepResult = { matches: GrepMatch[]; total: number; error: string | null };

export const executeGrep = (content: string, pattern: string): GrepResult => {
  let re: RegExp;
  try {
    re = new RegExp(pattern, 'gu');
  } catch (err) {
    return { matches: [], total: 0, error: `정규식 오류: ${String(err).slice(0, 120)}` };
  }
  const matches: GrepMatch[] = [];
  let total = 0;
  for (const m of content.matchAll(re)) {
    if (m[0].length === 0) break; // 빈 매치 무한 루프 방지
    total += 1;
    if (matches.length < GREP_MAX) {
      const start = m.index;
      const end = start + m[0].length;
      matches.push({
        start,
        end,
        text: m[0],
        context: content.slice(Math.max(0, start - CONTEXT), Math.min(content.length, end + CONTEXT)),
      });
    }
  }
  return { matches, total, error: null };
};
