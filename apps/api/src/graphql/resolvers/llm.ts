import Anthropic from '@anthropic-ai/sdk';
import * as Sentry from '@sentry/node';
import dedent from 'dedent';
import { Repeater } from 'graphql-yoga';
import { env } from '#/env.ts';
import { builder } from '../builder.ts';

const anthropic = new Anthropic({
  apiKey: env.CLOUDFLARE_API_KEY,
  baseURL: env.CLOUDFLARE_AIGATEWAY_URL,
});

type Feedback = {
  start: string;
  end: string;
  feedback: string;
};

const provideFeedbackTool: Anthropic.Tool = {
  name: 'provide_feedback',
  description: '현재 분석 구간에서 발견한 피드백 1건을 보고합니다. 피드백할 게 없으면 호출하지 마세요. 여러 건이면 여러 번 호출하세요.',
  input_schema: {
    type: 'object',
    properties: {
      start: { type: 'string', description: '구간 시작 문장 (현재 분석할 구간 내 원문 그대로)' },
      end: { type: 'string', description: '구간 끝 문장 (현재 분석할 구간 내 원문 그대로)' },
      feedback: { type: 'string', description: '피드백 본문' },
    },
    required: ['start', 'end', 'feedback'],
  },
};

const systemPrompt = dedent`
  당신은 글을 읽는 첫 번째 독자입니다.

  <context>
  현재 분석할 구간 앞뒤로 이야기 흐름이 제공됩니다.

  - [이전 내용]: 현재 구간 이전의 이야기 흐름
  - [현재 분석할 구간]: 현재 분석할 구간의 내용
  - [이후 내용]: 현재 구간 이후의 이야기 흐름

  현재 분석할 구간의 내용은 이전 내용과 이후 내용 사이에 있습니다.
  만약 피드백에서 이전 내용과 이후 내용을 언급해야 한다면, 이전 내용 혹은 이후 내용이라고 정확하게 언급하세요.
  단, 이전 내용 혹은 이후 내용을 언급할 때는 특수문자로 감싸지 마세요.
  </context>

  <principle>
  꼼꼼하게 읽고, 개선할 수 있는 부분을 찾아주세요.
  이 글의 이 부분에서 실제로 발생하는 구체적인 문제만 지적하세요.
  특정 유형의 피드백에 집착하지 마세요. 다양한 관점에서 균형 있게 살펴보세요.
  맞춤법, 문법, 띄어쓰기는 지적하지 마세요.
  신조어, 방언, 줄임말, 의도적인 어미 변형 등 작가의 문체적 선택은 존중하세요.
  언제나 전체 글을 읽고 있는 독자의 시선으로 작성하세요.
  </principle>

  <focus>
  - 독자로서 읽다가 걸리거나 몰입이 깨지는 부분
  - 장면 전환이 급하거나 어색한 부분
  - 누가 말하는지 헷갈리는 대화
  - 인물의 행동이나 반응이 맥락상 부자연스러운 부분
  - 설정이나 복선이 회수되지 않는 부분
  - 반복되는 단어나 표현
  - 문장이 너무 길거나 구조가 복잡해서 읽기 어려운 부분
  - 묘사가 부족하거나 과한 부분
  </focus>

  <examples>
  - "이 장면 전환이 갑작스러워요. 사이에 뭔가 있으면 자연스러울 것 같아요."
  - "대화가 길어지면서 누가 말하는지 헷갈려요. 중간에 행동 묘사를 넣으면 좋을 것 같아요."
  - "앞에서 언급한 OO이 여기서 다시 나오면 좋을 것 같은데, 그냥 지나가네요."
  - "이 문장이 좀 길어서 한 번에 읽기 어려워요. 나눠보면 어떨까요?"
  - "여기서 인물이 갑자기 태도가 바뀌는데, 이유가 좀 더 드러나면 좋겠어요."
  </examples>

  <tone>
  "~하면 어떨까요?", "~인 것 같아요"
  </tone>

  <output>
  분석 결과는 provide_feedback tool을 호출해서 보고하세요.
  피드백 1건당 한 번씩 호출하세요. 피드백할 게 없으면 호출하지 마세요.
  </output>
`;

const CHUNK_SIZE = 1000;

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

const summarizePrompt = dedent`
  다음 텍스트를 요약하세요.

  포함할 내용:
  - 등장인물과 그들의 관계
  - 주요 사건과 행동
  - 감정 변화와 분위기
  - 장소나 시간의 변화
  - 대화의 핵심 내용
  - 중요하거나 일반적이지 않은 단어나 용어

  없던 내용을 추측으로 만들어내거나, 내용을 왜곡하지 마세요. 최대한 원문을 그대로 보존하세요.
  마크다운 및 섹션 구분 없이 연속된 요약 텍스트만 출력하세요.

  300자 이내로 작성하세요.
`;

const summarizeChunk = async (chunkText: string, signal?: AbortSignal): Promise<string> => {
  const response = await anthropic.messages.create(
    {
      model: 'claude-haiku-4-5',
      max_tokens: 1024,
      system: summarizePrompt,
      messages: [{ role: 'user', content: `요약할 텍스트:\n\n${chunkText}` }],
    },
    { signal },
  );

  return response.content.find((b): b is Anthropic.TextBlock => b.type === 'text')?.text ?? '';
};

type ChunkContext = {
  precedingSummary: string;
  followingSummary: string;
  currentText: string;
};

const analyzeChunkWithContext = async (
  context: ChunkContext,
  onFeedback: (feedback: Feedback) => void,
  signal?: AbortSignal,
): Promise<void> => {
  const userContent = dedent`
    <이전 내용>
    ${context.precedingSummary || '(글의 시작 부분입니다)'}
    </이전 내용>

    <현재 분석할 구간>
    ${context.currentText}
    </현재 분석할 구간>

    <이후 내용>
    ${context.followingSummary || '(글의 마지막 부분입니다)'}
    </이후 내용>
  `;

  const stream = anthropic.messages.stream(
    {
      model: 'claude-sonnet-4-6',
      max_tokens: 16_384,
      thinking: { type: 'adaptive', display: 'omitted' },
      output_config: { effort: 'low' },
      system: systemPrompt,
      tools: [provideFeedbackTool],
      messages: [{ role: 'user', content: userContent }],
    },
    { signal },
  );

  stream.on('contentBlock', (block) => {
    if (block.type !== 'tool_use' || block.name !== 'provide_feedback') return;
    const input = block.input as Feedback;
    if (input.start && input.end && input.feedback) {
      onFeedback(input);
    }
  });

  await stream.finalMessage();
};

const LiteraryAnalysisProgress = builder.simpleObject('LiteraryAnalysisProgress', {
  fields: (t) => ({
    current: t.int(),
    total: t.int(),
    phase: t.string(),
  }),
});

const DocumentLiteraryFeedbackResult = builder.simpleObject('DocumentLiteraryFeedbackResult', {
  fields: (t) => ({
    nodeId: t.string(),
    startOffset: t.int(),
    endOffset: t.int(),
    startText: t.string(),
    endText: t.string(),
    feedback: t.string(),
  }),
});

type DocumentAnalysisPayload =
  | {
      type: 'feedback';
      data: { nodeId: string; startOffset: number; endOffset: number; startText: string; endText: string; feedback: string };
    }
  | { type: 'progress'; data: { current: number; total: number; phase: 'summarizing' | 'analyzing' } }
  | { type: 'complete' }
  | { type: 'error' };

const DocumentTextMappingInput = builder.inputType('DocumentTextMappingInput', {
  fields: (t) => ({
    nodeId: t.string(),
    textStart: t.int(),
    textEnd: t.int(),
    blockOffset: t.int(),
  }),
});

const createMapRangeForDocument = (
  text: string,
  mappings: { nodeId: string; textStart: number; textEnd: number; blockOffset: number }[],
) => {
  const findMapping = (position: number) => {
    let left = 0;
    let right = mappings.length - 1;

    while (left <= right) {
      const mid = (left + right) >> 1;
      const m = mappings[mid];

      if (position >= m.textStart && position < m.textEnd) {
        return m;
      }

      if (position < m.textStart) {
        right = mid - 1;
      } else {
        left = mid + 1;
      }
    }

    return;
  };

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

    const startMapping = findMapping(rangeStart);
    const endMapping = findMapping(rangeEnd - 1);

    if (!startMapping || !endMapping || startMapping.nodeId !== endMapping.nodeId) {
      return null;
    }

    return {
      nodeId: startMapping.nodeId,
      startOffset: startMapping.blockOffset + (rangeStart - startMapping.textStart),
      endOffset: startMapping.blockOffset + (rangeEnd - startMapping.textStart),
    };
  };
};

builder.subscriptionFields((t) => ({
  literaryAnalysisDocumentStream: t.withAuth({ session: true }).field({
    type: builder.simpleObject('DocumentLiteraryAnalysisPayload', {
      fields: (t) => ({
        type: t.string(),
        feedback: t.field({ type: DocumentLiteraryFeedbackResult, nullable: true }),
        progress: t.field({ type: LiteraryAnalysisProgress, nullable: true }),
      }),
    }),
    args: {
      text: t.arg.string(),
      mappings: t.arg({ type: [DocumentTextMappingInput] }),
    },
    subscribe: (_, args, ctx) => {
      const text = args.text;
      const mappings = args.mappings;

      return new Repeater<DocumentAnalysisPayload>(async (push, stop) => {
        const abortController = new AbortController();
        const signal = abortController.signal;

        ctx.c.req.raw.signal.addEventListener('abort', () => {
          abortController.abort();
          stop();
        });

        if (!text.trim()) {
          push({ type: 'complete' });
          stop();
          return;
        }

        const chunks = createChunks(text);
        const mapRange = createMapRangeForDocument(text, mappings);

        try {
          const summaries: string[] = [];
          await Promise.all(
            chunks.map(async (chunk, index) => {
              signal.throwIfAborted();
              const summary = await summarizeChunk(chunk.text, signal);
              summaries[index] = summary;
              push({
                type: 'progress',
                data: { current: summaries.filter(Boolean).length, total: chunks.length, phase: 'summarizing' },
              });
            }),
          );

          let analyzedCount = 0;
          push({
            type: 'progress',
            data: { current: 0, total: chunks.length, phase: 'analyzing' },
          });
          await Promise.all(
            chunks.map(async (chunk, i) => {
              signal.throwIfAborted();
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
                        nodeId: range.nodeId,
                        startOffset: range.startOffset,
                        endOffset: range.endOffset,
                        startText: feedback.start,
                        endText: feedback.end,
                        feedback: feedback.feedback,
                      },
                    });
                  }
                },
                signal,
              );

              analyzedCount++;
              push({
                type: 'progress',
                data: { current: analyzedCount, total: chunks.length, phase: 'analyzing' },
              });
            }),
          );

          push({ type: 'complete' });
        } catch (err) {
          if (!signal.aborted) {
            Sentry.captureException(err);
            console.error(err);
            push({ type: 'error' });
          }
        }

        stop();
      });
    },
    resolve: (payload: DocumentAnalysisPayload) => {
      return {
        type: payload.type,
        feedback: payload.type === 'feedback' ? payload.data : null,
        progress: payload.type === 'progress' ? payload.data : null,
      };
    },
  }),
}));
