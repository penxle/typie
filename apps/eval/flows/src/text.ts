import type OpenAI from 'openai';

const CHUNK_SIZE = 1000;
// 경계 탐색 폭. 좁으면(구 버전 200자) 긴 서술 문단에서 문장 중간이 잘리고, 그러면 분석 단계가
// 그 구간을 "미완성 문장"으로 오독한다 — 실측에서 오독 지적의 상당수가 여기서 나왔다.
const SEARCH_WINDOW = 400;

// 종결부호 뒤에 닫는 따옴표·괄호가 붙는 한국어 소설 관례를 포함한다("…했다." 뒤의 」, ' 등).
const SENTENCE_END = /[.!?。！？…][”’"'」』〉》)\]]*\s*/g;
// 문장 경계가 없을 때의 차선 — 최소한 어절 중간에서는 자르지 않는다.
const SOFT_BREAK = /[,、·:;)\]}」』]\s|\s/g;

// 캐시 키에 섞어 청킹 규칙이 바뀌면 옛 요약이 재사용되지 않게 한다.
export const CHUNK_VERSION = 2;

const lastMatchBefore = (text: string, pattern: RegExp, searchStart: number, end: number): number => {
  pattern.lastIndex = searchStart;
  let last = -1;
  let match;
  while ((match = pattern.exec(text)) && match.index <= end) {
    last = match.index + match[0].length;
  }
  return last;
};

export const createChunks = (text: string) => {
  const chunks: { text: string; start: number; end: number }[] = [];

  let pos = 0;
  while (pos < text.length) {
    let end = pos + CHUNK_SIZE;

    if (end < text.length) {
      const searchStart = Math.max(pos, end - SEARCH_WINDOW);
      let breakPoint = -1;

      for (let i = end; i >= searchStart; i--) {
        if (text[i] === '\n') {
          breakPoint = i + 1;
          break;
        }
      }

      if (breakPoint === -1) {
        breakPoint = lastMatchBefore(text, SENTENCE_END, searchStart, end);
      }

      if (breakPoint <= pos) {
        breakPoint = lastMatchBefore(text, SOFT_BREAK, searchStart, end);
      }

      if (breakPoint > pos) {
        end = breakPoint;
      }
    } else {
      end = text.length;
    }

    chunks.push({
      text: text.slice(pos, end),
      start: pos,
      end,
    });

    pos = end;
  }

  return chunks;
};

export type Feedback = {
  start: string;
  end: string;
  feedback: string;
  category?: string;
  polarity?: string;
};

export type SummaryStructured = {
  narrative: string;
  characters: string[];
  pov: string;
  tense: string;
  location: string;
  tone: string;
  // 장면 전환·회상 진입/복귀 등 구조 정보 — 구형 저장분에는 없다.
  transitions?: string;
};

// 별칭은 사용 조건이 있는 구조형과 구형 문자열이 공존한다 — 기존 저장분(stage_cache 등) 호환.
export type CharacterAlias = string | { alias: string; usage?: string };

export type MetaStructured = {
  narrator: { pov: string; reliability: string };
  setting: string;
  themes: string[];
  characters: { name: string; aliases: CharacterAlias[]; role: string; arc: string }[];
  structure: { label: string; summary: string; tone: string }[];
  style: string;
};

export const dedupCharacterCandidates = (summaries: SummaryStructured[]): string[] => {
  const seen = new Set<string>();
  const result: string[] = [];
  for (const s of summaries) {
    for (const name of s.characters ?? []) {
      const normalized = name.trim().replaceAll(/^["']|["']$/g, '');
      if (!normalized) continue;
      const key = normalized.toLowerCase();
      if (!seen.has(key)) {
        seen.add(key);
        result.push(normalized);
      }
    }
  }
  return result;
};

export const renderSummaryForMeta = (summary: SummaryStructured): string => {
  const characters = summary.characters ?? [];

  const meta1: string[] = [];
  if (characters.length > 0) meta1.push(`인물: ${characters.join(', ')}`);
  if (summary.pov) meta1.push(`시점: ${summary.pov}`);
  if (summary.tense) meta1.push(`시제: ${summary.tense}`);

  const meta2: string[] = [];
  if (summary.location) meta2.push(`장소: ${summary.location}`);
  if (summary.tone) meta2.push(`분위기: ${summary.tone}`);
  if (summary.transitions) meta2.push(`장면·시간 구조: ${summary.transitions}`);

  const lines: string[] = [];
  if (summary.narrative) lines.push(summary.narrative);
  if (meta1.length > 0) lines.push(meta1.map((m) => `[${m}]`).join(' '));
  if (meta2.length > 0) lines.push(meta2.map((m) => `[${m}]`).join(' '));

  return lines.join('\n');
};

// 인접 구간 컨텍스트 — META 입력과 동일한 렌더링을 쓴다. 요약의 구조 필드(시점·시제·장소·
// 장면·시간 구조)가 분석 단계의 경계 판정(전환 신호·회상)에 그대로 쓰이도록.
export const renderAdjacentSummary = (summary: SummaryStructured | undefined): string => {
  if (!summary) return '';
  return renderSummaryForMeta(summary);
};

export const renderMetaBlock = (meta: MetaStructured): string => {
  const characterLines = (meta.characters ?? []).map((c) => {
    const aliases = (c.aliases ?? []).map((a) => (typeof a === 'string' ? a : a.usage ? `${a.alias}: ${a.usage}` : a.alias));
    const aliasPart = aliases.length > 0 ? ` (${aliases.join('/')})` : '';
    return `- ${c.name ?? ''}${aliasPart}: ${c.role ?? ''}. ${c.arc ?? ''}`;
  });
  const structureLines = (meta.structure ?? []).map((s) => `- ${s.label ?? ''}: ${s.summary ?? ''} [${s.tone ?? ''}]`);

  return [
    '<작품 전체>',
    `서술 시점: ${meta.narrator?.pov ?? ''}`,
    `화자 신뢰성: ${meta.narrator?.reliability ?? ''}`,
    `배경: ${meta.setting ?? ''}`,
    `주제: ${(meta.themes ?? []).join(', ')}`,
    `문체: ${meta.style ?? ''}`,
    '',
    '등장인물:',
    ...characterLines,
    '',
    '구조:',
    ...structureLines,
    '</작품 전체>',
  ].join('\n');
};

export type Match = { index: number; length: number };

const escape = (s: string) => s.replaceAll(/[|\\{}()[\]^$+*?.]/g, String.raw`\$&`).replaceAll('-', String.raw`\x2d`);

export const fuzzyFindMatch = (haystack: string, needle: string, fromIndex: number): Match | null => {
  const trimmed = needle.trim();
  if (!trimmed) return null;
  const pattern = escape(trimmed).replaceAll(/\s+/g, String.raw`\s+`);
  const subStart = Math.max(0, fromIndex);
  const match = new RegExp(pattern).exec(haystack.slice(subStart));
  if (!match) return null;
  return { index: subStart + match.index, length: match[0].length };
};

// 모델이 자주 일으키는 인용 변형(따옴표 날조·스타일 변경, 공백 소실)을 흡수하는 최후 폴백용 정규화.
const isMatchIgnored = (ch: string) => ch === '"' || ch === '“' || ch === '”' || ch === "'" || ch === '‘' || ch === '’' || /\s/.test(ch);

const buildNormalizedIndex = (text: string) => {
  const kept: string[] = [];
  const map: number[] = [];
  let i = 0;
  while (i < text.length) {
    if (!isMatchIgnored(text[i])) {
      kept.push(text[i]);
      map.push(i);
    }
    i++;
  }
  return { normalized: kept.join(''), map };
};

const normalizeForMatch = (s: string) => {
  let out = '';
  for (const ch of s) {
    if (!isMatchIgnored(ch)) out += ch;
  }
  return out;
};

// 모델이 인용을 옮기다 남기는 잡음을 걷어낸다. 실측된 두 가지를 겨냥한다:
// ① 개행 직후 첫 한글 음절이 음이 같은 가나로 바뀌어 나온다(에→エ, 하→は, 무→む).
//    개행이 잦은 원고에서 집중적으로 터진다 — 원문에 가나가 한 글자도 없어도 발생한다.
// ② 인용 앞뒤에 원문에 없는 문장부호(…, — 등)가 덧붙는다.
// 정상 경로가 전부 실패한 뒤에만 쓰므로, 원문에 실제로 가나가 있는 인용은 이미 매칭돼 여기 오지 않는다.
const KANA = /[\u{3040}-\u{30FF}]/gu;
const EDGE_NOISE = /^[\s…—·.,'"‘’“”]+|[\s…—·,'"‘’“”]+$/gu;

export const repairQuote = (s: string): string => s.replaceAll(KANA, '').replaceAll(EDGE_NOISE, '');

export const createFindRange = (text: string) => {
  const { normalized, map } = buildNormalizedIndex(text);

  return (startText: string, endText: string, searchStart: number) => {
    const exactFind = (needle: string, from: number): Match | null => {
      const idx = text.indexOf(needle, from);
      return idx === -1 ? null : { index: idx, length: needle.length };
    };

    const normalizedFind = (needle: string, from: number): Match | null => {
      const n = normalizeForMatch(needle.trim());
      if (!n) return null;
      let lo = 0;
      while (lo < map.length && map[lo] < from) lo++;
      const idx = normalized.indexOf(n, lo);
      if (idx === -1) return null;
      const first = map[idx];
      const last = map[idx + n.length - 1];
      return { index: first, length: last + 1 - first };
    };

    const tryFinders = (find: (needle: string, from: number) => Match | null, head: string, tail: string) => {
      const start = find(head, searchStart);
      if (!start) return null;
      // end가 start 인용 범위 안의 문장이어도 유효한 앵커다 — 겹침을 허용한다.
      const end = find(tail, start.index);
      if (end) {
        return { rangeStart: start.index, rangeEnd: Math.max(start.index + start.length, end.index + end.length) };
      }
      // 모델이 두 인용을 원문 순서와 반대로 준 경우 — 앞쪽에서 다시 찾아 둘을 잇는다.
      const earlier = find(tail, searchStart);
      if (!earlier || earlier.index >= start.index) return null;
      return { rangeStart: earlier.index, rangeEnd: start.index + start.length };
    };

    const finders = [exactFind, (n: string, from: number) => fuzzyFindMatch(text, n, from), normalizedFind];
    const attempt = (head: string, tail: string) => {
      for (const find of finders) {
        const range = tryFinders(find, head, tail);
        if (range) return range;
      }
      return null;
    };

    const repairedStart = repairQuote(startText);
    const repairedEnd = repairQuote(endText);
    const range =
      attempt(startText, endText) ?? (repairedStart === startText && repairedEnd === endText ? null : attempt(repairedStart, repairedEnd));
    if (!range) {
      return null;
    }
    return range;
  };
};

export const extractJsonObjects = function* (buffer: string): Generator<string> {
  let depth = 0;
  let start = -1;
  let inString = false;
  let escapeNext = false;

  for (let i = 0; i < buffer.length; i++) {
    const ch = buffer[i];
    if (inString) {
      if (escapeNext) {
        escapeNext = false;
      } else if (ch === '\\') {
        escapeNext = true;
      } else if (ch === '"') {
        inString = false;
      }
      continue;
    }
    if (ch === '"') {
      inString = true;
    } else if (ch === '{') {
      if (depth === 0) start = i;
      depth++;
    } else if (ch === '}') {
      depth--;
      if (depth === 0 && start !== -1) {
        yield buffer.slice(start, i + 1);
        start = -1;
      }
    }
  }
};

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type ToolDescriptions = Record<string, any>;

export const buildFeedbackTool = (d: ToolDescriptions): OpenAI.Chat.Completions.ChatCompletionFunctionTool => ({
  type: 'function',
  function: {
    name: 'provide_feedback',
    description: d.tool,
    parameters: {
      type: 'object',
      properties: {
        start: { type: 'string', description: d.start },
        end: { type: 'string', description: d.end },
        feedback: { type: 'string', description: d.feedback },
        category: { type: 'string', description: d.category },
        polarity: { type: 'string', enum: ['issue', 'highlight'], description: d.polarity },
      },
      required: ['start', 'end', 'feedback', 'category', 'polarity'],
    },
  },
});

export const buildSummaryTool = (d: ToolDescriptions): OpenAI.Chat.Completions.ChatCompletionFunctionTool => ({
  type: 'function',
  function: {
    name: 'provide_summary',
    description: d.tool,
    parameters: {
      type: 'object',
      properties: {
        narrative: { type: 'string', description: d.narrative },
        characters: { type: 'array', items: { type: 'string' }, description: d.characters },
        pov: { type: 'string', description: d.pov },
        tense: { type: 'string', description: d.tense },
        location: { type: 'string', description: d.location },
        tone: { type: 'string', description: d.tone },
        transitions: { type: 'string', description: d.transitions },
      },
    },
  },
});

export const buildMetaTool = (d: ToolDescriptions): OpenAI.Chat.Completions.ChatCompletionFunctionTool => ({
  type: 'function',
  function: {
    name: 'provide_meta',
    description: d.tool,
    parameters: {
      type: 'object',
      properties: {
        narrator: {
          type: 'object',
          description: d.narrator.self,
          properties: {
            pov: { type: 'string', description: d.narrator.pov },
            reliability: { type: 'string', description: d.narrator.reliability },
          },
        },
        setting: { type: 'string', description: d.setting },
        themes: { type: 'array', items: { type: 'string' }, description: d.themes },
        characters: {
          type: 'array',
          description: d.characters.self,
          items: {
            type: 'object',
            properties: {
              name: { type: 'string', description: d.characters.name },
              aliases: {
                type: 'array',
                description: d.characters.aliases,
                items: {
                  type: 'object',
                  properties: {
                    alias: { type: 'string', description: d.characters.alias },
                    usage: {
                      type: 'string',
                      description: d.characters.aliasUsage,
                    },
                  },
                  required: ['alias'],
                },
              },
              role: { type: 'string', description: d.characters.role },
              arc: { type: 'string', description: d.characters.arc },
            },
          },
        },
        structure: {
          type: 'array',
          description: d.structure.self,
          items: {
            type: 'object',
            properties: {
              label: { type: 'string', description: d.structure.label },
              summary: { type: 'string', description: d.structure.summary },
              tone: { type: 'string', description: d.structure.tone },
            },
          },
        },
        style: { type: 'string', description: d.style },
      },
    },
  },
});
