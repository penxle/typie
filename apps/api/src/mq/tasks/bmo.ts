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

let schema: unknown | null = null;

// spell-checker:disable
const getSchemaQuery = `
WITH table_info AS (
  SELECT 
    t.table_name,
    obj_description(c.oid) as table_comment
  FROM information_schema.tables t
  LEFT JOIN pg_catalog.pg_class c ON c.relname = t.table_name AND c.relnamespace = (
    SELECT oid FROM pg_catalog.pg_namespace WHERE nspname = t.table_schema
  )
  WHERE t.table_schema = 'public' 
    AND t.table_type = 'BASE TABLE'
),
column_info AS (
  SELECT 
    c.table_name,
    c.column_name,
    c.data_type,
    c.is_nullable,
    c.column_default,
    c.ordinal_position,
    col_description(pgc.oid, c.ordinal_position) as column_comment,
    CASE 
      WHEN fk.constraint_name IS NOT NULL THEN 
        json_build_object(
          'table', fk.foreign_table_name,
          'column', fk.foreign_column_name
        )
      ELSE NULL
    END as foreign_key
  FROM information_schema.columns c
  LEFT JOIN pg_catalog.pg_class pgc ON pgc.relname = c.table_name AND pgc.relnamespace = (
    SELECT oid FROM pg_catalog.pg_namespace WHERE nspname = c.table_schema
  )
  LEFT JOIN (
    SELECT
      kcu.table_name,
      kcu.column_name,
      ccu.table_name AS foreign_table_name,
      ccu.column_name AS foreign_column_name,
      tc.constraint_name
    FROM information_schema.table_constraints tc
    JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name
    JOIN information_schema.constraint_column_usage ccu ON ccu.constraint_name = tc.constraint_name
    WHERE tc.constraint_type = 'FOREIGN KEY'
  ) fk ON fk.table_name = c.table_name AND fk.column_name = c.column_name
  WHERE c.table_schema = 'public'
),
index_info AS (
  SELECT 
    tablename as table_name,
    indexname,
    indexdef
  FROM pg_indexes
  WHERE schemaname = 'public'
),
enum_info AS (
  SELECT 
    t.typname as enum_name,
    array_agg(e.enumlabel ORDER BY e.enumsortorder) as enum_values
  FROM pg_type t
  JOIN pg_enum e ON t.oid = e.enumtypid
  JOIN pg_namespace n ON n.oid = t.typnamespace
  WHERE n.nspname = 'public'
    AND t.typtype = 'e'
  GROUP BY t.typname
)
SELECT json_build_object(
  'tables', (
    SELECT json_agg(
      json_build_object(
        'table_name', t.table_name,
        'table_comment', t.table_comment,
        'columns', (
          SELECT json_agg(
            json_build_object(
              'column_name', c.column_name,
              'data_type', c.data_type,
              'is_nullable', c.is_nullable = 'YES',
              'column_default', c.column_default,
              'column_comment', c.column_comment,
              'foreign_key', c.foreign_key
            ) ORDER BY c.ordinal_position
          )
          FROM column_info c
          WHERE c.table_name = t.table_name
        ),
        'indexes', (
          SELECT json_agg(
            json_build_object(
              'index_name', i.indexname,
              'index_def', i.indexdef
            )
          )
          FROM index_info i
          WHERE i.table_name = t.table_name
        )
      ) ORDER BY t.table_name
    )
    FROM table_info t
  ),
  'enums', (
    SELECT json_agg(
      json_build_object(
        'enum_name', e.enum_name,
        'enum_values', e.enum_values
      ) ORDER BY e.enum_name
    )
    FROM enum_info e
  )
) as schema;
`;
// spell-checker:enable

const getDatabaseSchema = async () => {
  try {
    if (schema) {
      return {
        success: true,
        data: schema,
      };
    }

    const result = await sql.begin('read only', async (sql) => {
      const [row] = await sql.unsafe(getSchemaQuery);
      return row?.schema || { tables: [], enums: [] };
    });

    schema = result;

    return {
      success: true,
      data: result,
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
        name: 'get_database_schema',
        description:
          '데이터베이스 스키마 정보를 가져옵니다. 테이블 구조, 컬럼 정보, 데이터 타입, 외래키 관계, 인덱스, 커스텀 ENUM 타입 등을 포함합니다. SQL 쿼리 작성 전에 먼저 이 도구를 사용하세요.',
        input_schema: {
          type: 'object',
          properties: {},
          required: [],
        },
      },
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
      - 먼저 get_database_schema 도구로 스키마 정보를 확인
      - execute_sql_query 도구를 사용하여 PostgreSQL 데이터베이스 쿼리 실행
      - 읽기 전용 트랜잭션으로 안전하게 실행 (INSERT, UPDATE, DELETE 불가)
      - 실시간 데이터 조회 및 분석 가능
      - 스키마 정보를 기반으로 정확한 테이블과 컬럼명을 사용해 쿼리 작성
      - 모든 쿼리는 Asia/Seoul 타임존을 지정해 작성
      - 필요시 여러 쿼리를 연속 실행하여 심층 분석 가능

      execute_sql_query 도구 사용 시 주의사항:
      - description 파라미터는 반드시 구체적이고 의미 있는 설명으로 작성
      - 쿼리가 조회하는 데이터, 사용하는 테이블, 조인 관계, 목적을 명확히 설명
      - 좋은 예시:
        * "users 테이블에서 최근 7일간 신규 가입한 ACTIVE 상태 사용자 수 조회"
        * "subscriptions와 plans 테이블을 조인하여 이번 달 구독 매출 총액 계산"
        * "posts와 post_reactions 테이블을 조인하여 reaction 수 기준 상위 10개 인기 게시물 분석"
        * "entities와 posts를 조인하고 post_contents와 연결하여 특정 사이트의 공개 게시물 목록 조회"
        * "users, sites, entities를 차례로 조인하여 특정 유저가 작성한 모든 엔티티 개수 집계"

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

          if (tool.name === 'get_database_schema') {
            statusMessage = '📊 데이터베이스 스키마 확인 중...';
            await updateSlackMessage(responseText + '\n\n' + statusMessage, true);

            toolResult = await getDatabaseSchema();
          } else if (tool.name === 'execute_sql_query') {
            const toolInput = tool.input as { query: string; description: string };
            const truncatedQuery = toolInput.query.length > 1000 ? toolInput.query.slice(0, 1000) + '...' : toolInput.query;
            statusMessage = `🔍 데이터베이스 조회 중: ${toolInput.description}\n\`\`\`\n${truncatedQuery}\n\`\`\``;
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
