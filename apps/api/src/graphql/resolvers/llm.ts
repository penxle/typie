import { Anthropic } from '@anthropic-ai/sdk';
import { Node } from '@tiptap/pm/model';
import { Repeater } from 'graphql-yoga';
import pMap from 'p-map';
import { rapidhash } from 'rapidhash-js';
import { redis } from '@/cache';
import { env } from '@/env';
import { schema, textSerializers } from '@/pm';
import { builder } from '../builder';

const anthropic = new Anthropic({ apiKey: env.ANTHROPIC_API_KEY });

type DraftFeedback = {
  text: string;
  feedback: string;
};

type ValidatedFeedback = {
  text: string;
  feedback: string;
  score: number;
  confidence: number;
};

const MIN_CONFIDENCE = 0.5;

const LiteraryFeedbackResult = builder.simpleObject('LiteraryFeedbackResult', {
  fields: (t) => ({
    from: t.int(),
    to: t.int(),
    text: t.string(),
    feedback: t.string(),
    score: t.int(),
  }),
});

const systemPrompt = `당신은 소설 작법에 정통한 문학 편집자입니다. 작가의 산문을 개선하도록 돕습니다.

<core_principle>
좋은 산문은 독자를 이야기 속에 몰입시킵니다. 나쁜 산문은 독자를 밀어냅니다.
몰입을 방해하는 모든 요소가 피드백 대상입니다.
</core_principle>

<examples_of_craft_issues>
- 감정/상태 직접 서술: "그는 슬펐다" → 행동과 감각으로 보여주기
- 클리셰: 과용되어 힘을 잃은 표현들
- 약한 동사+부사: "빠르게 달렸다" → "질주했다"
- 필터 워드: "~을 보았다", "~을 느꼈다"가 만드는 거리감
- 중복: 대사가 이미 전달하는 감정을 태그로 또 설명
- 과잉 수식: 불필요한 형용사/부사의 나열
- 추상적 서술: 감각적 디테일 없이 요약만

(이 외에도 산문의 질을 떨어뜨리는 작법 문제가 있다면 지적하세요)
</examples_of_craft_issues>

<what_is_not_a_problem>
- 담백한 서술: "문이 열렸다", "비가 내렸다"
- 의도적 문체: 작가가 효과를 위해 선택한 표현
- 주관적 선호: "더 좋을 수 있다" 수준의 사소한 것
</what_is_not_a_problem>

<output>
JSON Lines (한 줄에 하나):
{"text":"원문","feedback":"무엇이 문제인지 + 왜 문제인지 + 개선 방향"}

피드백할 게 없으면 아무것도 출력하지 마세요.
</output>`;

const MAX_CHUNK_SIZE = 1000;
const MAX_CONCURRENCY = 5;
const CACHE_TTL = 60 * 60 * 24;
const SENTENCE_PATTERN = /([.!?。！？]+\s*)/g;

const extractTextAndMappings = (body: unknown) => {
  const node = Node.fromJSON(schema, body);

  let text = '';
  let textOffset = 0;
  const textNodeMappings: { textStart: number; textEnd: number; pmStart: number }[] = [];

  node.nodesBetween(0, node.content.size, (childNode, pos, parent, index) => {
    const textSerializer = textSerializers[childNode.type.name];
    if (textSerializer) {
      if (parent) {
        const range = { from: 0, to: node.content.size };
        const serialized = textSerializer({ node: childNode, pos, parent, index, range });
        text += serialized;
        textOffset += serialized.length;
      }
      return false;
    }

    if (childNode.isBlock && pos > 0) {
      text += '\n';
      textOffset += 1;
    }

    if (childNode.isText && childNode.text) {
      const content = childNode.text;
      textNodeMappings.push({
        textStart: textOffset,
        textEnd: textOffset + content.length,
        pmStart: pos,
      });
      text += content;
      textOffset += content.length;
    }
  });

  return { text, textNodeMappings };
};

const createChunks = (text: string) => {
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

  return chunks;
};

const createMapRange = (text: string, textNodeMappings: { textStart: number; textEnd: number; pmStart: number }[]) => {
  return (quotedText: string, searchStart = 0) => {
    const textStart = text.indexOf(quotedText, searchStart);
    if (textStart === -1) {
      return null;
    }
    const textEnd = textStart + quotedText.length;

    const startMapping = textNodeMappings.find((m) => textStart >= m.textStart && textStart < m.textEnd);
    const endMapping = textNodeMappings.find((m) => textEnd > m.textStart && textEnd <= m.textEnd);

    if (!startMapping || !endMapping) {
      return null;
    }

    const from = startMapping.pmStart + (textStart - startMapping.textStart);
    const to = endMapping.pmStart + (textEnd - endMapping.textStart);

    return { from, to };
  };
};

const validationPrompt = `당신은 문학 작법 피드백을 검증합니다.

<principle>
기본적으로 valid: true입니다. 명백히 억지인 경우만 false로 판정하세요.
</principle>

<invalid_cases>
- "문이 열렸다" 같은 담백한 서술을 비판
- 구체적 근거 없이 "더 좋게 쓸 수 있다"만 말함
</invalid_cases>

<scoring>
1-3: 심각 (감정 직접 서술, 클리셰)
4-6: 중간 (개선 여지)
7-10: 경미
</scoring>

<output>
{"valid":true/false,"score":1-10,"confidence":0.5-1.0}
</output>`;

const validateFeedback = async (originalText: string, draft: DraftFeedback): Promise<ValidatedFeedback | null> => {
  const response = await anthropic.messages.create({
    model: 'claude-sonnet-4-5-20250929',
    max_tokens: 100,
    system: validationPrompt,
    messages: [
      {
        role: 'user',
        content: `원문:\n"${originalText}"\n\n비평 대상: "${draft.text}"\n비평 내용: "${draft.feedback}"`,
      },
    ],
  });

  const text = response.content[0].type === 'text' ? response.content[0].text : '';

  try {
    const jsonMatch = text.match(/\{[\s\S]*\}/);
    if (!jsonMatch) {
      console.log('[literary:validation] no JSON found:', text);
      return null;
    }

    const result = JSON.parse(jsonMatch[0]) as { valid: boolean; score: number; confidence: number };
    console.log('[literary:validation] result:', { draft: draft.text, result });

    if (result.valid && result.confidence >= MIN_CONFIDENCE) {
      return {
        text: draft.text,
        feedback: draft.feedback,
        score: result.score,
        confidence: result.confidence,
      };
    }
  } catch (err) {
    console.log('[literary:validation] parse error:', text, err);
  }

  return null;
};

const analyzeChunkStreaming = async (chunkText: string, onFeedback: (feedback: ValidatedFeedback) => void): Promise<void> => {
  const hash = rapidhash(systemPrompt + validationPrompt + chunkText);
  const cacheKey = `literary:${hash}`;

  const cached = await redis.get(cacheKey);
  if (cached) {
    const feedbacks = JSON.parse(cached) as ValidatedFeedback[];
    for (const feedback of feedbacks) {
      onFeedback(feedback);
    }
    return;
  }

  const feedbacks: ValidatedFeedback[] = [];

  const stream = anthropic.messages.stream({
    model: 'claude-sonnet-4-5-20250929',
    max_tokens: 4096,
    system: systemPrompt,
    messages: [
      {
        role: 'user',
        content: `다음 텍스트를 문학적으로 분석해주세요:\n\n${chunkText}`,
      },
    ],
  });

  let buffer = '';

  for await (const event of stream) {
    if (event.type === 'content_block_delta' && event.delta.type === 'text_delta') {
      buffer += event.delta.text;

      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;

        try {
          const draft = JSON.parse(trimmed) as DraftFeedback;
          console.log('[literary:draft] parsed:', draft);
          if (draft.text && draft.feedback) {
            const validated = await validateFeedback(chunkText, draft);
            if (validated) {
              feedbacks.push(validated);
              onFeedback(validated);
            }
          }
        } catch {
          // skip invalid JSON
        }
      }
    }
  }

  if (buffer.trim()) {
    try {
      const draft = JSON.parse(buffer.trim()) as DraftFeedback;
      if (draft.text && draft.feedback) {
        const validated = await validateFeedback(chunkText, draft);
        if (validated) {
          feedbacks.push(validated);
          onFeedback(validated);
        }
      }
    } catch {
      // skip invalid JSON
    }
  }

  await redis.setex(cacheKey, CACHE_TTL, JSON.stringify(feedbacks));
};

type AnalysisPayload =
  | { type: 'feedback'; data: { from: number; to: number; text: string; feedback: string; score: number } }
  | { type: 'progress'; data: { current: number; total: number } }
  | { type: 'complete' }
  | { type: 'error' };

const LiteraryAnalysisProgress = builder.simpleObject('LiteraryAnalysisProgress', {
  fields: (t) => ({
    current: t.int(),
    total: t.int(),
  }),
});

builder.subscriptionFields((t) => ({
  literaryAnalysisStream: t.withAuth({ session: true }).field({
    type: builder.simpleObject('LiteraryAnalysisPayload', {
      fields: (t) => ({
        type: t.string(),
        feedback: t.field({ type: LiteraryFeedbackResult, nullable: true }),
        progress: t.field({ type: LiteraryAnalysisProgress, nullable: true }),
      }),
    }),
    args: {
      body: t.arg({ type: 'JSON' }),
    },
    subscribe: (_, args, ctx) => {
      const { text, textNodeMappings } = extractTextAndMappings(args.body);

      return new Repeater<AnalysisPayload>(async (push, stop) => {
        ctx.c.req.raw.signal.addEventListener('abort', () => {
          stop();
        });

        if (!text.trim()) {
          push({ type: 'complete' });
          stop();
          return;
        }

        const chunks = createChunks(text);
        const mapRange = createMapRange(text, textNodeMappings);
        let completedChunks = 0;

        try {
          await pMap(
            chunks,
            async (chunk) => {
              await analyzeChunkStreaming(chunk.text, (feedback) => {
                const range = mapRange(feedback.text, chunk.start);

                if (range) {
                  push({
                    type: 'feedback',
                    data: {
                      from: range.from,
                      to: range.to,
                      text: feedback.text,
                      feedback: feedback.feedback,
                      score: feedback.score,
                    },
                  });
                }
              });

              completedChunks++;
              push({
                type: 'progress',
                data: { current: completedChunks, total: chunks.length },
              });
            },
            { concurrency: MAX_CONCURRENCY },
          );

          push({ type: 'complete' });
        } catch (err) {
          console.error('[literary:analysis] error:', err);
          push({ type: 'error' });
        }

        stop();
      });
    },
    resolve: (payload: AnalysisPayload) => {
      return {
        type: payload.type,
        feedback: payload.type === 'feedback' ? payload.data : null,
        progress: payload.type === 'progress' ? payload.data : null,
      };
    },
  }),
}));
