import '@typie/lib/dayjs';

import { Anthropic } from '@anthropic-ai/sdk';
import { WebClient } from '@slack/web-api';
import dayjs from 'dayjs';
import dedent from 'dedent';
import postgres from 'postgres';
import { env } from '@/env';
import { defineJob } from '../types';

type SlackAppMentionEventPayload = {
  user: string;
  text: string;
  ts: string;
  thread_ts?: string;
  channel: string;
  event_ts: string;
};

const sql = postgres(env.DATABASE_URL, {
  ssl: { rejectUnauthorized: false },
  prepare: false,
  max: 5,
  max_lifetime: 10 * 60,
  connection: {
    statement_timeout: 60_000,
    TimeZone: 'Asia/Seoul',
  },
});

const anthropic = new Anthropic({ apiKey: env.ANTHROPIC_API_KEY });
const slack = new WebClient(env.SLACK_BOT_TOKEN);

const executeQuery = async (query: string) => {
  try {
    const result = await sql.begin('read only', async (sql) => {
      return await sql.unsafe(query);
    });

    return {
      success: true,
      count: result.length,
      rows: [...result],
    };
  } catch (err) {
    return {
      success: false,
      error: err instanceof Error ? err.message : String(err),
    };
  }
};

const SLACK_UPDATE_INTERVAL = 1000;
const MIN_UPDATE_CHARS = 50;

export const ProcessBmoMentionJob = defineJob('bmo:process-mention', async (event: SlackAppMentionEventPayload) => {
  let messageTs: string | undefined;
  let lastUpdateTime = Date.now();
  let lastUpdateText = '';
  let updateTimer: NodeJS.Timeout | null = null;

  const updateSlackMessage = async (text: string, force = false) => {
    if (!messageTs) return;

    const now = Date.now();
    const timeSinceUpdate = now - lastUpdateTime;
    const charsSinceUpdate = text.length - lastUpdateText.length;

    if (force || (timeSinceUpdate >= SLACK_UPDATE_INTERVAL && charsSinceUpdate >= MIN_UPDATE_CHARS)) {
      await slack.chat.update({
        channel: event.channel,
        ts: messageTs,
        text: text || '💭 생각 중...',
      });
      lastUpdateTime = now;
      lastUpdateText = text;
    }
  };

  const scheduleUpdate = (text: string) => {
    if (updateTimer) clearTimeout(updateTimer);
    updateTimer = setTimeout(() => updateSlackMessage(text), SLACK_UPDATE_INTERVAL);
  };

  try {
    const text = event.text.replaceAll(/<@[^>]+>/g, '').trim();

    if (!text) {
      await slack.chat.postMessage({
        channel: event.channel,
        thread_ts: event.thread_ts || event.ts,
        text: '안녕하세요! 무엇을 도와드릴까요?',
      });

      return;
    }

    await slack.reactions.add({
      channel: event.channel,
      timestamp: event.ts,
      name: 'eyes',
    });

    const initialMessage = await slack.chat.postMessage({
      channel: event.channel,
      thread_ts: event.thread_ts || event.ts,
      text: '💭 생각 중...',
      reply_broadcast: !event.thread_ts,
    });

    messageTs = initialMessage.ts;

    const conversation = await slack.conversations.replies({
      channel: event.channel,
      ts: event.thread_ts || event.ts,
      inclusive: true,
      limit: 10,
    });

    const messages: Anthropic.MessageParam[] = [];
    if (conversation.messages) {
      for (const msg of conversation.messages) {
        if (msg.ts === event.ts || !msg.text || !msg.user) {
          continue;
        }

        const role = msg.user === event.user ? 'user' : 'assistant';
        const text = msg.text.replaceAll(/<@[^>]+>/g, '').trim();

        if (text) {
          messages.push({ role, content: text });
        }
      }
    }

    messages.push({ role: 'user', content: text });

    const tools: Anthropic.Tool[] = [
      {
        name: 'execute_sql_query',
        description:
          'PostgreSQL 데이터베이스에서 읽기 전용 트랜잭션으로 쿼리를 실행합니다. SELECT, WITH, SHOW, EXPLAIN 등 읽기 작업만 가능합니다.',
        input_schema: {
          type: 'object',
          properties: {
            query: {
              type: 'string',
              description: 'SQL SELECT 쿼리. 시간 관련 쿼리는 Asia/Seoul 타임존을 사용하세요.',
            },
            description: {
              type: 'string',
              description: '이 쿼리가 무엇을 조회하는지에 대한 간단한 설명',
            },
          },
          required: ['query', 'description'],
        },
      },
      {
        name: 'get_current_time',
        description: '현재 시간을 한국 시간(Asia/Seoul)으로 반환합니다. 날짜와 시간 관련 분석 시 참조용으로 사용하세요.',
        input_schema: {
          type: 'object',
          properties: {},
          required: [],
        },
      },
    ];

    const system = dedent`
      당신은 타이피 개발팀의 데이터 분석 AI 어시스턴트 "비모" 입니다.
      비모는 타이피의 데이터베이스에 접근하여 데이터를 분석하고 인사이트를 제공합니다.
      비모는 Slack 메시지를 통해 사용자와 대화합니다.

      역할:
      - 데이터베이스 쿼리를 통한 데이터 추출 및 분석
      - 비즈니스 인사이트 도출 및 제공
      - 사용자 행동 패턴 분석
      - 성장 지표 및 KPI 모니터링
      - 데이터 기반 의사결정 지원

      데이터베이스 접근:
      - execute_sql_query 도구를 사용하여 PostgreSQL 데이터베이스 쿼리 실행
      - 읽기 전용 트랜잭션으로 안전하게 실행 (INSERT, UPDATE, DELETE 불가)
      - 실시간 데이터 조회 및 분석 가능
      - DB 스키마를 직접 분석해 필요한 테이블과 컬럼을 찾아 쿼리 작성 
      - 모든 쿼리는 Asia/Seoul 타임존을 지정해 작성
      - 필요시 여러 쿼리를 연속 실행하여 심층 분석 가능

      시간 정보:
      - get_current_time 도구로 현재 한국 시간 확인 가능
      - "오늘", "이번 주", "이번 달" 같은 상대적 시간 표현 처리 시 활용

      응답 가이드라인:
      - 한국어로 친근하고 전문적으로 소통
      - 데이터를 시각적으로 이해하기 쉽게 표현
      - 요청받지 않은 추가적인 분석 및 제안 금지

      Slack mrkdwn 포맷:
      - *굵은 글씨* (별표 하나)
      - _기울임_ (언더스코어)
      - ~취소선~ (물결표)
      - \`인라인 코드\` (백틱)
      - \`\`\`코드 블록\`\`\` (백틱 3개)
      - > 인용구 (꺽쇠)
      - • 글머리 기호 (불릿 포인트)

      주의: Slack은 **굵은** 같은 이중 별표를 지원하지 않음
    `;

    const maxIterations = 50;
    const accMessages = [...messages];

    for (let iteration = 0; iteration < maxIterations; iteration++) {
      let responseText = '';
      let hasToolUse = false;
      const toolsToExecute: { id: string; name: string; input: unknown }[] = [];

      const stream = anthropic.messages.stream({
        model: 'claude-opus-4-20250514',
        max_tokens: 10_000,
        messages: accMessages,
        system,
        tools,
      });

      for await (const chunk of stream) {
        if (chunk.type === 'content_block_start') {
          if (chunk.content_block.type === 'text') {
            responseText = '';
          } else if (chunk.content_block.type === 'tool_use') {
            hasToolUse = true;
            toolsToExecute.push({
              id: chunk.content_block.id,
              name: chunk.content_block.name,
              input: {},
            });
          }
        } else if (chunk.type === 'content_block_delta') {
          if (chunk.delta.type === 'text_delta') {
            responseText += chunk.delta.text;
            scheduleUpdate(responseText);
          }
        } else if (chunk.type === 'content_block_stop' && chunk.index < toolsToExecute.length) {
          const finalMessage = await stream.finalMessage();
          const toolBlock = finalMessage.content[chunk.index];
          if (toolBlock.type === 'tool_use') {
            toolsToExecute[chunk.index].input = toolBlock.input;
          }
        }
      }

      if (responseText && !hasToolUse) {
        await updateSlackMessage(responseText, true);
      }

      if (hasToolUse) {
        const toolResults: Anthropic.MessageParam[] = [];

        for (const tool of toolsToExecute) {
          let toolResult: unknown;
          let statusMessage = '';

          if (tool.name === 'execute_sql_query') {
            const toolInput = tool.input as { query: string; description: string };
            statusMessage = `🔍 데이터베이스 조회 중: ${toolInput.description}`;
            await updateSlackMessage(responseText + '\n\n' + statusMessage, true);

            toolResult = await executeQuery(toolInput.query);
          } else if (tool.name === 'get_current_time') {
            statusMessage = '⏰ 현재 시간 확인 중...';
            await updateSlackMessage(responseText + '\n\n' + statusMessage, true);

            const now = dayjs.kst();
            toolResult = {
              success: true,
              current_time_ko_kr: now.format('YYYY년 MM월 DD일 dddd HH시 mm분 ss초'),
              current_time_iso8601: now.toISOString(),
            };
          }

          toolResults.push({
            role: 'user' as const,
            content: [
              {
                type: 'tool_result' as const,
                tool_use_id: tool.id,
                content: JSON.stringify(toolResult),
              },
            ],
          });
        }

        const finalMessage = await stream.finalMessage();
        accMessages.push(
          {
            role: 'assistant' as const,
            content: finalMessage.content,
          },
          ...toolResults,
        );
      } else {
        break;
      }

      if (iteration === maxIterations - 1) {
        await updateSlackMessage(responseText || '지금은 응답을 할 수 없어요.', true);
      }
    }

    if (updateTimer) {
      clearTimeout(updateTimer);
    }

    await slack.reactions.add({
      channel: event.channel,
      timestamp: event.ts,
      name: 'white_check_mark',
    });

    await slack.reactions.remove({
      channel: event.channel,
      timestamp: event.ts,
      name: 'eyes',
    });
  } catch (err) {
    if (updateTimer) {
      clearTimeout(updateTimer);
    }

    if (messageTs) {
      await slack.chat.update({
        channel: event.channel,
        ts: messageTs,
        text: `오류가 발생했어요.\n\`\`\`${err instanceof Error ? err.message : String(err)}\`\`\``,
      });
    }

    await slack.reactions.add({
      channel: event.channel,
      timestamp: event.ts,
      name: 'x',
    });

    await slack.reactions.remove({
      channel: event.channel,
      timestamp: event.ts,
      name: 'eyes',
    });

    throw err;
  }
});
