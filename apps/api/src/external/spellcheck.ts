// spell-checker:words cand

import * as Sentry from '@sentry/node';
import { XMLParser } from 'fast-xml-parser';
import DOMPurify from 'isomorphic-dompurify';
import ky from 'ky';
import pMap from 'p-map';
import { rapidhash } from 'rapidhash-js';
import { redis } from '@/cache';
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
const MAX_CONCURRENCY = 100;

const ALLOWED_CHARS = /^[\u{AC00}-\u{D7AF}\u{3131}-\u{318E}A-Za-z0-9\s.,!?:;()[\]"'/\\@#$%&*+=_~`{}<>|^。、「」『』！？…·ㆍ-]$/u;
const SENTENCE_PATTERN = /([.!?。！？]+\s*)/g;

const normalize = (text: string) => {
  const removed: { pos: number; len: number }[] = [];
  let normalized = '';

  for (let i = 0; i < text.length; ) {
    const char = text[i];
    const code = text.codePointAt(i) || 0;

    if (i + 1 < text.length) {
      const nextCode = text.codePointAt(i + 1) || 0;
      const isSurrogatePair = code >= 0xd8_00 && code <= 0xdb_ff;
      const isVariationSelector = nextCode >= 0xfe_00 && nextCode <= 0xfe_0f;
      if (isSurrogatePair || isVariationSelector) {
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

  const map = (offset: number, isLast = false) => {
    let originalPos = 0;
    let normalizedPos = 0;

    for (let i = 0; i < text.length; ) {
      const isRemoved = removed.find((r) => r.pos === i);

      if (isRemoved) {
        if (offset === normalizedPos && isLast) {
          return originalPos;
        }
        originalPos += isRemoved.len;
        i += isRemoved.len;
      } else {
        if (normalizedPos === offset) {
          return originalPos;
        }
        normalizedPos++;
        originalPos++;
        i++;
      }
    }

    return originalPos;
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

  while (offset < text.length) {
    const nextNewline = text.indexOf('\n', offset);
    const paragraphEnd = nextNewline === -1 ? text.length : nextNewline;
    const paragraph = text.slice(offset, paragraphEnd);

    if (paragraph.trim().length > 0) {
      paragraphs.push({
        text: paragraph,
        start: offset,
        end: paragraphEnd,
      });
    }

    offset = paragraphEnd + 1;
  }

  let chunkEndOffset = 0;

  for (const paragraph of paragraphs) {
    if (chunk.length + (chunk ? 1 : 0) + paragraph.text.length <= MAX_CHUNK_SIZE) {
      if (chunk) {
        chunk += '\n' + paragraph.text;
        chunkEndOffset = paragraph.end;
      } else {
        chunk = paragraph.text;
        chunkStartOffset = paragraph.start;
        chunkEndOffset = paragraph.end;
      }
    } else {
      if (chunk && chunkEndOffset > 0) {
        chunks.push({
          text: text.slice(chunkStartOffset, chunkEndOffset),
          start: chunkStartOffset,
          end: chunkEndOffset,
        });
      }

      if (paragraph.text.length > MAX_CHUNK_SIZE) {
        let sentenceChunk = '';
        let sentenceStartOffset = paragraph.start;
        let sentenceEndOffset = paragraph.start;
        let endOffset = 0;

        const pattern = new RegExp(SENTENCE_PATTERN.source, SENTENCE_PATTERN.flags);
        let match;

        while ((match = pattern.exec(paragraph.text))) {
          const matchEndOffset = match.index + match[0].length;
          const sentence = paragraph.text.slice(endOffset, matchEndOffset);

          if (sentenceChunk.length + sentence.length <= MAX_CHUNK_SIZE) {
            sentenceChunk += sentence;
            sentenceEndOffset = paragraph.start + matchEndOffset;
          } else {
            if (sentenceChunk) {
              chunks.push({
                text: text.slice(sentenceStartOffset, sentenceEndOffset),
                start: sentenceStartOffset,
                end: sentenceEndOffset,
              });
            }
            sentenceChunk = sentence;
            sentenceStartOffset = paragraph.start + endOffset;
            sentenceEndOffset = paragraph.start + matchEndOffset;
          }
          endOffset = matchEndOffset;
        }

        const remaining = paragraph.text.slice(endOffset);
        if (remaining) {
          if (sentenceChunk.length + remaining.length <= MAX_CHUNK_SIZE) {
            sentenceChunk += remaining;
            sentenceEndOffset = paragraph.end;
          } else {
            if (sentenceChunk) {
              chunks.push({
                text: text.slice(sentenceStartOffset, sentenceEndOffset),
                start: sentenceStartOffset,
                end: sentenceEndOffset,
              });
            }
            sentenceChunk = remaining;
            sentenceStartOffset = paragraph.start + endOffset;
            sentenceEndOffset = paragraph.end;
          }
        }

        if (sentenceChunk) {
          chunks.push({
            text: text.slice(sentenceStartOffset, sentenceEndOffset),
            start: sentenceStartOffset,
            end: sentenceEndOffset,
          });
        }

        chunk = '';
        chunkEndOffset = 0;
      } else {
        chunk = paragraph.text;
        chunkStartOffset = paragraph.start;
        chunkEndOffset = paragraph.end;
      }
    }
  }

  if (chunk && chunkEndOffset > 0) {
    chunks.push({
      text: text.slice(chunkStartOffset, chunkEndOffset),
      start: chunkStartOffset,
      end: chunkEndOffset,
    });
  }

  const results = await pMap(
    chunks,
    async (chunk) => {
      try {
        const chunkText = chunk.text;
        const normalized = normalize(chunkText);

        const hash = rapidhash(normalized.text);
        const key = `spellcheck:${hash}`;

        let xml = await redis.get(key);
        if (!xml) {
          xml = await ky
            .post(env.SPELLCHECK_URL, {
              headers: { 'x-api-key': env.SPELLCHECK_API_KEY },
              json: { sentence: normalized.text },
            })
            .text();

          await redis.setex(key, 60 * 60 * 24, xml);
        }

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

        return errors.map((error) => {
          const start = normalized.map(Number(error.m_nStart));
          const end = normalized.map(Number(error.m_nEnd), true);

          return {
            index: Number(error.nErrorIdx),
            start: chunk.start + start,
            end: chunk.start + end,
            context: chunkText.slice(start, end),
            corrections:
              error.CandWordList && Number(error.CandWordList.m_nCount) > 0
                ? [error.CandWordList.CandWord].flat().filter((x) => x !== undefined)
                : [],
            explanation: DOMPurify.sanitize(error.Help?.['#text'] ?? '', { ALLOWED_TAGS: ['br'] }),
            type: errorTypes[Number(error.Help?.nCorrectMethod)],
          };
        });
      } catch (err) {
        Sentry.captureException(err);
        return [];
      }
    },
    { concurrency: MAX_CONCURRENCY },
  );

  return results.flat();
};
