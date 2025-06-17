// spell-checker:words cand

import * as Sentry from '@sentry/node';
import { XMLParser } from 'fast-xml-parser';
import ky from 'ky';
import pMap from 'p-map';
import { env } from '@/env';

const errorTypes = [
  'no-error',
  'morpheme-analysis',
  'misused-word',
  'multi-word',
  'semantic-style',
  'punctuation',
  'statistical-spacing',
  'english-misuse',
  'tagging',
  'compound-noun',
  'context-spacing',
] as const;

export type SpellingError = {
  index: number;
  start: number;
  end: number;
  context: string;
  corrections: string[];
  explanation: string;
  type: (typeof errorTypes)[number];
};

type CheckSpellResponse = {
  PnuNlpSpeller?: {
    PnuErrorWordList?: {
      Error?: string | { msg?: string };
      PnuErrorWord?: PnuErrorWord | PnuErrorWord[];
    };
  };
};

type PnuErrorWord = {
  nErrorIdx: string;
  m_nStart: string;
  m_nEnd: string;
  CandWordList?: {
    m_nCount: string;
    CandWord?: string | string[];
  };
  Help?: {
    '#text': string;
    nCorrectMethod?: string;
  };
};

const parser = new XMLParser({ ignoreAttributes: false, attributeNamePrefix: '' });
const MAX_CHUNK_SIZE = 500;
const ALLOWED_CHARS = /^[\u{AC00}-\u{D7AF}\u{3131}-\u{318E}A-Za-z0-9\s.,!?:;()[\]"'/\\@#$%&*+=_~`{}<>|^。、「」『』！？…·ㆍ-]$/u;
const SENTENCE_PATTERN = /([.!?。！？]+\s*)/g;

const normalize = (text: string) => {
  const removed: { pos: number; len: number }[] = [];
  let normalized = '';

  for (let i = 0; i < text.length; ) {
    const char = text[i];
    const code = text.codePointAt(i) || 0;

    if (code >= 0xd8_00 && code <= 0xdb_ff && i + 1 < text.length) {
      const nextCode = text.codePointAt(i + 1) || 0;
      if (nextCode >= 0xdc_00 && nextCode <= 0xdf_ff) {
        removed.push({ pos: i, len: 2 });
        i += 2;
        continue;
      }
    }

    if (ALLOWED_CHARS.test(char)) {
      normalized += char;
      i++;
    } else {
      removed.push({ pos: i, len: 1 });
      i++;
    }
  }

  const map = (offset: number) => {
    let adjustment = 0;
    let normalizedPos = 0;

    for (let i = 0; i < text.length && normalizedPos < offset; i++) {
      const r = removed.find((r) => r.pos === i);
      if (r) {
        adjustment += r.len;
        i += r.len - 1;
      } else {
        normalizedPos++;
      }
    }

    return offset + adjustment;
  };

  return { text: normalized, map };
};

export const check = async (text: string) => {
  const normalized = normalize(text);

  if (!normalized.text.trim()) return [];

  const chunks: { text: string; start: number; end: number }[] = [];
  let chunk = '';
  let chunkStartOffset = 0;
  let offset = 0;

  const paragraphs: { text: string; start: number; end: number }[] = [];

  while (offset < normalized.text.length) {
    const nextNewline = normalized.text.indexOf('\n', offset);
    const paragraphEnd = nextNewline === -1 ? normalized.text.length : nextNewline;
    const paragraph = normalized.text.slice(offset, paragraphEnd);

    if (paragraph.trim().length > 0) {
      paragraphs.push({
        text: paragraph,
        start: offset,
        end: paragraphEnd,
      });
    }

    offset = paragraphEnd + 1;
  }

  for (const paragraph of paragraphs) {
    if (chunk.length + (chunk ? 1 : 0) + paragraph.text.length <= MAX_CHUNK_SIZE) {
      if (chunk) {
        chunk += '\n' + paragraph.text;
      } else {
        chunk = paragraph.text;
        chunkStartOffset = paragraph.start;
      }
    } else {
      if (chunk) {
        chunks.push({
          text: chunk,
          start: normalized.map(chunkStartOffset),
          end: normalized.map(chunkStartOffset + chunk.length),
        });
      }

      if (paragraph.text.length > MAX_CHUNK_SIZE) {
        let sentenceChunk = '';
        let sentenceStartOffset = paragraph.start;
        let endOffset = 0;

        const pattern = new RegExp(SENTENCE_PATTERN.source, SENTENCE_PATTERN.flags);
        let match;

        while ((match = pattern.exec(paragraph.text))) {
          const sentenceEndOffset = match.index + match[0].length;
          const sentence = paragraph.text.slice(endOffset, sentenceEndOffset);

          if (sentenceChunk.length + sentence.length <= MAX_CHUNK_SIZE) {
            sentenceChunk += sentence;
          } else {
            if (sentenceChunk) {
              chunks.push({
                text: sentenceChunk,
                start: normalized.map(sentenceStartOffset),
                end: normalized.map(sentenceStartOffset + sentenceChunk.length),
              });
            }
            sentenceChunk = sentence;
            sentenceStartOffset = paragraph.start + endOffset;
          }
          endOffset = sentenceEndOffset;
        }

        const remaining = paragraph.text.slice(endOffset);
        if (remaining) {
          if (sentenceChunk.length + remaining.length <= MAX_CHUNK_SIZE) {
            sentenceChunk += remaining;
          } else {
            if (sentenceChunk) {
              chunks.push({
                text: sentenceChunk,
                start: normalized.map(sentenceStartOffset),
                end: normalized.map(sentenceStartOffset + sentenceChunk.length),
              });
            }
            sentenceChunk = remaining;
            sentenceStartOffset = paragraph.start + endOffset;
          }
        }

        if (sentenceChunk) {
          chunks.push({
            text: sentenceChunk,
            start: normalized.map(sentenceStartOffset),
            end: normalized.map(sentenceStartOffset + sentenceChunk.length),
          });
        }

        chunk = '';
      } else {
        chunk = paragraph.text;
        chunkStartOffset = paragraph.start;
      }
    }
  }

  if (chunk) {
    chunks.push({
      text: chunk,
      start: normalized.map(chunkStartOffset),
      end: normalized.map(chunkStartOffset + chunk.length),
    });
  }

  const results = await pMap(
    chunks,
    async (chunk) => {
      try {
        const xml = await ky
          .post(env.SPELLCHECK_URL, {
            headers: { 'x-api-key': env.SPELLCHECK_API_KEY },
            json: { sentence: chunk.text },
          })
          .text();

        const resp = parser.parse(xml) as CheckSpellResponse;
        const errorList = resp.PnuNlpSpeller?.PnuErrorWordList;

        if (errorList?.Error) {
          const msg = typeof errorList.Error === 'string' ? errorList.Error : errorList.Error.msg;
          if (msg !== '문법 및 철자 오류가 발견되지 않았습니다.') {
            Sentry.captureException(new Error(`Spellcheck API error: ${msg}`));
          }

          return [];
        }

        if (!errorList?.PnuErrorWord) return [];

        const errors = [errorList.PnuErrorWord].flat() as PnuErrorWord[];
        const normalized = normalize(text.slice(chunk.start, chunk.end));

        return errors.map((error) => {
          const start = normalized.map(Number(error.m_nStart));
          const end = normalized.map(Number(error.m_nEnd));

          return {
            index: Number(error.nErrorIdx),
            start: chunk.start + start,
            end: chunk.start + end,
            context: text.slice(chunk.start + start, chunk.start + end),
            corrections:
              error.CandWordList && Number(error.CandWordList.m_nCount) > 0
                ? [error.CandWordList.CandWord].flat().filter((x) => x !== undefined)
                : [],
            explanation: error.Help?.['#text'] ?? '',
            type: errorTypes[Number(error.Help?.nCorrectMethod)],
          };
        });
      } catch (err) {
        Sentry.captureException(err);
        return [];
      }
    },
    { concurrency: 10 },
  );

  return results.flat();
};
