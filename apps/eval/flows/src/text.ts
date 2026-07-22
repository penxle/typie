import type OpenAI from 'openai';

const CHUNK_SIZE = 1000;

export const createChunks = (text: string) => {
  const chunks: { text: string; start: number; end: number }[] = [];

  let pos = 0;
  while (pos < text.length) {
    let end = pos + CHUNK_SIZE;

    if (end < text.length) {
      const searchStart = Math.max(pos, end - 200);
      let breakPoint = -1;

      for (let i = end; i >= searchStart; i--) {
        if (text[i] === '\n') {
          breakPoint = i + 1;
          break;
        }
      }

      if (breakPoint === -1) {
        const sentencePattern = /[.!?。！？]\s*/g;
        sentencePattern.lastIndex = searchStart;
        let lastMatch = -1;
        let match;

        while ((match = sentencePattern.exec(text)) && match.index <= end) {
          lastMatch = match.index + match[0].length;
        }

        if (lastMatch > pos) {
          breakPoint = lastMatch;
        }
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
};

export type SummaryStructured = {
  narrative: string;
  characters: string[];
  pov: string;
  tense: string;
  location: string;
  tone: string;
};

export type MetaStructured = {
  narrator: { pov: string; reliability: string };
  setting: string;
  themes: string[];
  characters: { name: string; aliases: string[]; role: string; arc: string }[];
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

  const lines: string[] = [];
  if (summary.narrative) lines.push(summary.narrative);
  if (meta1.length > 0) lines.push(meta1.map((m) => `[${m}]`).join(' '));
  if (meta2.length > 0) lines.push(meta2.map((m) => `[${m}]`).join(' '));

  return lines.join('\n');
};

export const renderMetaBlock = (meta: MetaStructured): string => {
  const characterLines = (meta.characters ?? []).map((c) => {
    const aliases = c.aliases ?? [];
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

export const createFindRange = (text: string) => {
  return (startText: string, endText: string, searchStart: number) => {
    const exactFind = (needle: string, from: number): Match | null => {
      const idx = text.indexOf(needle, from);
      return idx === -1 ? null : { index: idx, length: needle.length };
    };

    const tryFinders = (find: (needle: string, from: number) => Match | null) => {
      const start = find(startText, searchStart);
      if (!start) return null;
      const endFrom = startText === endText ? start.index : start.index + start.length;
      const end = find(endText, endFrom);
      if (!end) return null;
      return { rangeStart: start.index, rangeEnd: end.index + end.length };
    };

    const range = tryFinders(exactFind) ?? tryFinders((n, from) => fuzzyFindMatch(text, n, from));
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
      },
      required: ['start', 'end', 'feedback'],
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
              aliases: { type: 'array', items: { type: 'string' }, description: d.characters.aliases },
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
