// 원고 접근 도구의 실행기 — 스펙 §4. 원고는 프롬프트 접두부가 아니라 이 두 도구로 접근한다.

export const READ_CAP = 4000;

// UTF-16 유닛 경계 슬라이스는 서로게이트 쌍(이모지 등)을 반으로 자를 수 있다. 잘린 반쪽이
// 도구 결과에 들어가면 다음 API 요청의 JSON 직렬화가 400으로 죽고, 캐시 리플레이로 재실행마다
// 재현된다(2026-07-30 실측). 경계가 쌍 중간이면 한 유닛 물러난다 — start에선 쌍이 포함되고
// end에선 제외되므로, 연속 창 읽기에서 쌍은 다음 창으로 온전히 넘어간다.
const snapToPair = (content: string, index: number): number => {
  if (index <= 0 || index >= content.length) return index;
  const unit = content.codePointAt(index) ?? 0;
  const prev = content.codePointAt(index - 1) ?? 0;
  // 직전 유닛에서 시작하는 코드포인트가 BMP 밖이면 index는 그 쌍의 한가운데다.
  return unit >= 0xdc_00 && unit <= 0xdf_ff && prev > 0xff_ff ? index - 1 : index;
};

export type ReadResult = { start: number; end: number; text: string; truncated: boolean };

export const executeRead = (content: string, start: number, end: number): ReadResult => {
  const s = snapToPair(content, Math.max(0, Math.min(Math.floor(start), content.length)));
  const requested = Math.max(s, Math.min(Math.floor(end), content.length));
  const truncated = requested - s > READ_CAP;
  const e = snapToPair(content, truncated ? s + READ_CAP : requested);
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
        context: content.slice(
          snapToPair(content, Math.max(0, start - CONTEXT)),
          snapToPair(content, Math.min(content.length, end + CONTEXT)),
        ),
      });
    }
  }
  return { matches, total, error: null };
};
