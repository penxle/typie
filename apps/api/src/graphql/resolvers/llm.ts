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

type Feedback = {
  start: string;
  end: string;
  feedback: string;
};

const LiteraryFeedbackResult = builder.simpleObject('LiteraryFeedbackResult', {
  fields: (t) => ({
    from: t.int(),
    to: t.int(),
    startText: t.string(),
    endText: t.string(),
    feedback: t.string(),
  }),
});

const systemPrompt = `당신은 글을 읽는 첫 번째 독자입니다.

<context>
현재 분석할 구간 앞뒤로 요약이 제공됩니다.
- [이전 내용 요약]: 현재 구간 이전의 이야기 흐름
- [이후 내용 요약]: 현재 구간 이후의 이야기 흐름
</context>

<principle>
꼼꼼하게 읽고, 개선할 수 있는 부분을 찾아주세요.
이 글의 이 부분에서 실제로 발생하는 구체적인 문제만 지적하세요.
</principle>

<focus>
- 감정이나 상태를 직접 서술하는 대신 보여줄 수 있는 부분
- 대화 태그가 과잉 설명하는 부분
- 장면 전환이 급하거나 어색한 부분
- 누가 말하는지 헷갈리는 대화
- 설정이나 복선이 회수되지 않는 부분
- 반복되는 단어나 표현
- 독자로서 읽다가 걸리는 부분
</focus>

<examples>
- "여기서 '슬펐다'고 직접 말하는 대신, 행동이나 묘사로 보여주면 더 와닿을 것 같아요."
- "'화난 목소리로 말했다' 대신 대사 자체로 감정을 드러내면 어떨까요?"
- "대화가 길어지면서 누가 말하는지 헷갈려요. 중간에 행동 묘사를 넣으면 좋을 것 같아요."
- "이 장면 전환이 갑작스러워요. 사이에 뭔가 있으면 자연스러울 것 같아요."
- "앞에서 언급한 OO이 여기서 다시 나오면 좋을 것 같은데, 그냥 지나가네요."
</examples>

<tone>
"~하면 어떨까요?", "~인 것 같아요"
</tone>

<output>
JSON Lines:
{"start":"구간 시작 문장","end":"구간 끝 문장","feedback":"피드백"}

피드백할 게 없으면 출력하지 마세요.
</output>`;

const CHUNK_SIZE = 1000;
const SUMMARY_MAX_TOKENS = 300;
const CACHE_TTL = 60 * 60 * 24;
const SUMMARIZE_CONCURRENCY = 5;

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

const summarizePrompt = `다음 텍스트를 요약하세요.

포함할 내용:
- 등장인물과 그들의 관계
- 주요 사건과 행동
- 감정 변화와 분위기
- 장소나 시간의 변화
- 대화의 핵심 내용
- 중요하거나 일반적이지 않은 단어나 용어

300자 이내로 작성하세요.
`;

const summarizeChunk = async (chunkText: string): Promise<string> => {
  const hash = rapidhash(summarizePrompt + chunkText);
  const cacheKey = `literary:summary:${hash}`;

  const cached = await redis.get(cacheKey);
  if (cached) {
    return cached;
  }

  const response = await anthropic.messages.create({
    model: 'claude-haiku-4-5-20251001',
    max_tokens: SUMMARY_MAX_TOKENS,
    system: summarizePrompt,
    messages: [
      {
        role: 'user',
        content: `요약할 텍스트:\n\n${chunkText}`,
      },
    ],
  });

  const summary = response.content[0].type === 'text' ? response.content[0].text : '';
  await redis.setex(cacheKey, CACHE_TTL, summary);

  return summary;
};

const createMapRange = (text: string, textNodeMappings: { textStart: number; textEnd: number; pmStart: number }[]) => {
  return (startText: string, endText: string, searchStart = 0) => {
    const rangeStart = text.indexOf(startText, searchStart);
    if (rangeStart === -1) {
      return null;
    }

    const endSearchStart = startText === endText ? rangeStart : rangeStart + startText.length;
    const endIndex = text.indexOf(endText, endSearchStart);
    if (endIndex === -1) {
      return null;
    }
    const rangeEnd = endIndex + endText.length;

    const startMapping = textNodeMappings.find((m) => rangeStart >= m.textStart && rangeStart < m.textEnd);
    const endMapping = textNodeMappings.find((m) => rangeEnd > m.textStart && rangeEnd <= m.textEnd);

    if (!startMapping || !endMapping) {
      return null;
    }

    const from = startMapping.pmStart + (rangeStart - startMapping.textStart);
    const to = endMapping.pmStart + (rangeEnd - endMapping.textStart);

    return { from, to };
  };
};

type ChunkContext = {
  precedingSummary: string;
  followingSummary: string;
  currentText: string;
};

const analyzeChunkWithContext = async (context: ChunkContext, onFeedback: (feedback: Feedback) => void): Promise<void> => {
  const hash = rapidhash(systemPrompt + JSON.stringify(context) + '1');
  const cacheKey = `literary:feedback:${hash}`;

  const cached = await redis.get(cacheKey);
  if (cached) {
    const feedbacks = JSON.parse(cached) as Feedback[];
    for (const feedback of feedbacks) {
      onFeedback(feedback);
    }
    return;
  }

  const feedbacks: Feedback[] = [];

  let userContent = '';
  if (context.precedingSummary) {
    userContent += `[이전 내용 요약]\n${context.precedingSummary}\n\n`;
  }
  userContent += `[현재 분석할 구간]\n${context.currentText}`;
  if (context.followingSummary) {
    userContent += `\n\n[이후 내용 요약]\n${context.followingSummary}`;
  }

  const stream = anthropic.messages.stream({
    model: 'claude-sonnet-4-5-20250929',
    max_tokens: 16_000,
    thinking: {
      type: 'enabled',
      budget_tokens: 10_000,
    },
    system: systemPrompt,
    messages: [
      {
        role: 'user',
        content: userContent,
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
          const feedback = JSON.parse(trimmed) as Feedback;
          if (feedback.start && feedback.end && feedback.feedback) {
            feedbacks.push(feedback);
            onFeedback(feedback);
          }
        } catch {
          // skip invalid JSON
        }
      }
    }
  }

  if (buffer.trim()) {
    try {
      const feedback = JSON.parse(buffer.trim()) as Feedback;
      if (feedback.start && feedback.end && feedback.feedback) {
        feedbacks.push(feedback);
        onFeedback(feedback);
      }
    } catch {
      // skip invalid JSON
    }
  }

  await redis.setex(cacheKey, CACHE_TTL, JSON.stringify(feedbacks));
};

type AnalysisPayload =
  | { type: 'feedback'; data: { from: number; to: number; startText: string; endText: string; feedback: string } }
  | { type: 'progress'; data: { current: number; total: number; phase: 'summarizing' | 'analyzing' } }
  | { type: 'complete' }
  | { type: 'error' };

const LiteraryAnalysisProgress = builder.simpleObject('LiteraryAnalysisProgress', {
  fields: (t) => ({
    current: t.int(),
    total: t.int(),
    phase: t.string(),
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

        try {
          const summaries: string[] = [];
          await pMap(
            chunks,
            async (chunk, index) => {
              const summary = await summarizeChunk(chunk.text);
              summaries[index] = summary;
              push({
                type: 'progress',
                data: { current: summaries.filter(Boolean).length, total: chunks.length, phase: 'summarizing' },
              });
            },
            { concurrency: SUMMARIZE_CONCURRENCY },
          );

          for (let i = 0; i < chunks.length; i++) {
            const chunk = chunks[i];

            push({
              type: 'progress',
              data: { current: i, total: chunks.length, phase: 'analyzing' },
            });

            const precedingSummary = summaries.slice(0, i).join('\n\n');
            const followingSummary = summaries.slice(i + 1).join('\n\n');

            await analyzeChunkWithContext(
              {
                precedingSummary,
                followingSummary,
                currentText: chunk.text,
              },
              (feedback) => {
                const range = mapRange(feedback.start, feedback.end, chunk.start);

                if (range) {
                  push({
                    type: 'feedback',
                    data: {
                      from: range.from,
                      to: range.to,
                      startText: feedback.start,
                      endText: feedback.end,
                      feedback: feedback.feedback,
                    },
                  });
                }
              },
            );
          }

          push({ type: 'complete' });
        } catch {
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
