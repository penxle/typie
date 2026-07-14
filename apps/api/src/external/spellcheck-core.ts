// spell-checker:words cand

import { XMLParser } from 'fast-xml-parser';
import DOMPurify from 'isomorphic-dompurify';
import pMap from 'p-map';
import { rapidhash } from 'rapidhash-js';

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

type SpellcheckDependencies = {
  cacheGet: (key: string) => Promise<string | null>;
  cacheSet: (key: string, ttlSeconds: number, value: string) => Promise<void>;
  requestXml: (text: string, signal?: AbortSignal) => Promise<string>;
};

type SpellcheckChunkStage = 'cache-read' | 'provider-request' | 'cache-write' | 'parse' | 'provider-response';

class SpellcheckChunkError extends Error {
  override readonly name = 'SpellcheckChunkError';

  constructor(stage: SpellcheckChunkStage, chunkIndex: number, chunkCount: number, cause: unknown, httpStatus?: number) {
    const status = httpStatus === undefined ? '' : ` (${httpStatus})`;
    super(`Spellcheck chunk ${chunkIndex + 1}/${chunkCount} failed at ${stage}${status}`, { cause });
  }
}

type Chunk = { text: string; start: number; end: number };

const parser = new XMLParser({
  ignoreAttributes: false,
  attributeNamePrefix: '',
});
const MAX_CHUNK_SIZE = 500;
const MAX_CONCURRENCY = 100;
const CACHE_TTL_SECONDS = 60 * 60 * 24;
const NO_ERROR_SENTINEL = '문법 및 철자 오류가 발견되지 않았습니다.';

const ALLOWED_CHARS = /^[\u{AC00}-\u{D7AF}\u{3131}-\u{318E}A-Za-z0-9\s.,!?:;()[\]"'/\\@#$%&*+=_~`{}<>|^。、「」『』“”‘’！？…·ㆍ-]$/u;
const SENTENCE_PATTERN = /([.!?。！？]+\s*)/g;

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value);

const toPlainTextExplanation = (value: string) => {
  const fragment = DOMPurify.sanitize(value, {
    ALLOWED_TAGS: ['br'],
    ALLOWED_ATTR: [],
    RETURN_DOM_FRAGMENT: true,
  }) as DocumentFragment;

  let plainText = '';
  for (const node of fragment.childNodes) {
    if (node.nodeType === 3) {
      plainText += node.textContent ?? '';
    } else if (node.nodeType === 1 && (node as Element).tagName === 'BR') {
      plainText += '\n';
    } else {
      plainText += node.textContent ?? '';
    }
  }
  return plainText;
};

const normalize = (text: string) => {
  const removed: { pos: number; len: number }[] = [];
  let normalized = '';

  for (let i = 0; i < text.length;) {
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
    if (ALLOWED_CHARS.test(char)) normalized += char;
    else removed.push({ pos: i, len: 1 });
    i += 1;
  }

  const map = (offset: number, isLast = false) => {
    let originalPos = 0;
    let normalizedPos = 0;
    let removedIndex = 0;
    for (let i = 0; i < text.length;) {
      const nextRemoved = removed[removedIndex];
      if (nextRemoved?.pos === i) {
        if (offset === normalizedPos && isLast) return originalPos;
        originalPos += nextRemoved.len;
        i += nextRemoved.len;
        removedIndex += 1;
      } else {
        if (normalizedPos === offset) return originalPos;
        normalizedPos += 1;
        originalPos += 1;
        i += 1;
      }
    }
    return originalPos;
  };

  return { text: normalized, map };
};

const splitChunks = (text: string): Chunk[] => {
  const chunks: Chunk[] = [];
  const paragraphs: Chunk[] = [];
  let offset = 0;
  while (offset < text.length) {
    const nextNewline = text.indexOf('\n', offset);
    const paragraphEnd = nextNewline === -1 ? text.length : nextNewline;
    const paragraph = text.slice(offset, paragraphEnd);
    if (paragraph.trim().length > 0) paragraphs.push({ text: paragraph, start: offset, end: paragraphEnd });
    offset = paragraphEnd + 1;
  }

  let chunk = '';
  let chunkStartOffset = 0;
  let chunkEndOffset = 0;
  const flush = () => {
    if (chunk && chunkEndOffset > 0) {
      chunks.push({
        text: text.slice(chunkStartOffset, chunkEndOffset),
        start: chunkStartOffset,
        end: chunkEndOffset,
      });
    }
    chunk = '';
    chunkEndOffset = 0;
  };

  for (const paragraph of paragraphs) {
    if (chunk.length + (chunk ? 1 : 0) + paragraph.text.length <= MAX_CHUNK_SIZE) {
      if (chunk) chunk += `\n${paragraph.text}`;
      else {
        chunk = paragraph.text;
        chunkStartOffset = paragraph.start;
      }
      chunkEndOffset = paragraph.end;
      continue;
    }

    flush();
    if (paragraph.text.length <= MAX_CHUNK_SIZE) {
      chunk = paragraph.text;
      chunkStartOffset = paragraph.start;
      chunkEndOffset = paragraph.end;
      continue;
    }

    let sentenceChunk = '';
    let sentenceStartOffset = paragraph.start;
    let sentenceEndOffset = paragraph.start;
    let endOffset = 0;
    const pattern = new RegExp(SENTENCE_PATTERN.source, SENTENCE_PATTERN.flags);
    for (let match = pattern.exec(paragraph.text); match; match = pattern.exec(paragraph.text)) {
      const matchEndOffset = match.index + match[0].length;
      const sentence = paragraph.text.slice(endOffset, matchEndOffset);
      if (sentenceChunk.length + sentence.length <= MAX_CHUNK_SIZE) sentenceChunk += sentence;
      else {
        if (sentenceChunk) {
          chunks.push({
            text: text.slice(sentenceStartOffset, sentenceEndOffset),
            start: sentenceStartOffset,
            end: sentenceEndOffset,
          });
        }
        sentenceChunk = sentence;
        sentenceStartOffset = paragraph.start + endOffset;
      }
      sentenceEndOffset = paragraph.start + matchEndOffset;
      endOffset = matchEndOffset;
    }

    const remaining = paragraph.text.slice(endOffset);
    if (remaining) {
      if (sentenceChunk.length + remaining.length <= MAX_CHUNK_SIZE) sentenceChunk += remaining;
      else {
        if (sentenceChunk) {
          chunks.push({
            text: text.slice(sentenceStartOffset, sentenceEndOffset),
            start: sentenceStartOffset,
            end: sentenceEndOffset,
          });
        }
        sentenceChunk = remaining;
        sentenceStartOffset = paragraph.start + endOffset;
      }
      sentenceEndOffset = paragraph.end;
    }
    if (sentenceChunk) {
      chunks.push({
        text: text.slice(sentenceStartOffset, sentenceEndOffset),
        start: sentenceStartOffset,
        end: sentenceEndOffset,
      });
    }
  }
  flush();
  return chunks;
};

const mapChunkResponse = (parsed: unknown, chunk: Chunk, normalized: ReturnType<typeof normalize>): SpellingError[] => {
  const response = parsed as CheckSpellResponse;
  const errorList = response.PnuNlpSpeller?.PnuErrorWordList;

  if (errorList?.Error) {
    const message = typeof errorList.Error === 'string' ? errorList.Error : errorList.Error.msg;
    if (message === NO_ERROR_SENTINEL) return [];
    throw new Error(`Spellcheck API error: ${message}`);
  }
  if (!errorList?.PnuErrorWord) return [];

  return [errorList.PnuErrorWord].flat().map((error) => {
    const normalizedStart = Number(error.m_nStart);
    const normalizedEnd = Number(error.m_nEnd);
    const start = normalized.map(normalizedStart);
    const end = normalized.map(normalizedEnd, true);

    return {
      index: Number(error.nErrorIdx),
      start: chunk.start + start,
      end: chunk.start + end,
      context: chunk.text.slice(start, end),
      corrections:
        error.CandWordList && Number(error.CandWordList.m_nCount) > 0
          ? [error.CandWordList.CandWord].flat().filter((candidate) => candidate !== undefined)
          : [],
      explanation: toPlainTextExplanation(error.Help?.['#text'] ?? ''),
      type: errorTypes[Number(error.Help?.nCorrectMethod)],
    };
  });
};

const errorName = (error: unknown): string | undefined => (isRecord(error) && typeof error.name === 'string' ? error.name : undefined);

const isAbort = (error: unknown, signal?: AbortSignal) =>
  signal?.aborted === true && (Object.is(error, signal.reason) || errorName(error) === 'AbortError');

const httpStatusOf = (error: unknown): number | undefined => {
  if (!isRecord(error) || !isRecord(error.response)) return undefined;
  return typeof error.response.status === 'number' ? error.response.status : undefined;
};

const asChunkError = (
  error: unknown,
  stage: SpellcheckChunkStage,
  chunkIndex: number,
  chunkCount: number,
  signal?: AbortSignal,
): SpellcheckChunkError => {
  if (isAbort(error, signal)) throw error;
  return new SpellcheckChunkError(stage, chunkIndex, chunkCount, error, httpStatusOf(error));
};

export const createSpellcheck =
  (dependencies: SpellcheckDependencies) =>
  async (text: string, signal?: AbortSignal): Promise<SpellingError[]> => {
    if (!normalize(text).text.trim()) return [];
    signal?.throwIfAborted();

    const chunks = splitChunks(text);
    const results = await pMap(
      chunks,
      async (chunk, chunkIndex) => {
        signal?.throwIfAborted();
        const normalized = normalize(chunk.text);
        const key = `spellcheck:${rapidhash(normalized.text)}`;

        let xml: string | null;
        try {
          xml = await dependencies.cacheGet(key);
        } catch (err) {
          throw asChunkError(err, 'cache-read', chunkIndex, chunks.length, signal);
        }
        signal?.throwIfAborted();

        if (!xml) {
          let providerXml: string;
          try {
            providerXml = await dependencies.requestXml(normalized.text, signal);
          } catch (err) {
            throw asChunkError(err, 'provider-request', chunkIndex, chunks.length, signal);
          }
          xml = providerXml;
          signal?.throwIfAborted();
          try {
            await dependencies.cacheSet(key, CACHE_TTL_SECONDS, providerXml);
          } catch (err) {
            throw asChunkError(err, 'cache-write', chunkIndex, chunks.length, signal);
          }
          signal?.throwIfAborted();
        }

        let parsed: unknown;
        try {
          parsed = parser.parse(xml);
        } catch (err) {
          throw asChunkError(err, 'parse', chunkIndex, chunks.length, signal);
        }
        try {
          return mapChunkResponse(parsed, chunk, normalized);
        } catch (err) {
          throw asChunkError(err, 'provider-response', chunkIndex, chunks.length, signal);
        }
      },
      { concurrency: MAX_CONCURRENCY },
    );
    return results.flat();
  };
