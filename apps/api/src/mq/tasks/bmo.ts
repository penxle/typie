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
  if (!schema) {
    schema = await sql.begin('read only', async (sql) => {
      const [row] = await sql.unsafe(getSchemaQuery);
      return row?.schema || { tables: [], enums: [] };
    });
  }

  return schema;
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
    if (!updateTimer) {
      updateTimer = setTimeout(async () => {
        await updateSlackMessage(text);
        updateTimer = null;
      }, SLACK_UPDATE_INTERVAL);
    }
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
              description: 'SQL 쿼리 문자열. SQL 쿼리 상단에 주석(-- 또는 /* */)으로 설명을 포함하세요.',
            },
          },
          required: ['query'],
        },
      },
    ];

    const dbSchema = await getDatabaseSchema();

    const system = dedent`
      # 시스템 정보
      현재 시간: ${dayjs.kst().format('YYYY년 MM월 DD일 dddd HH시 mm분 ss초')} (Asia/Seoul)

      # 기본 정보
      당신은 "비모(BMO)"입니다.
      - 역할: 타이피 개발팀의 데이터 분석 AI 어시스턴트
      - 목적: PostgreSQL 데이터베이스 쿼리를 통한 데이터 분석 및 인사이트 제공
      - 소통 채널: Slack 메시지
      - 언어: 한국어 (친근하고 전문적인 톤)

      # 핵심 제약사항
      1. 읽기 전용 데이터베이스 접근 (INSERT, UPDATE, DELETE 불가)
      2. 분당 최대 10만 토큰 제한
      3. 모든 쿼리는 Asia/Seoul 타임존 사용
      4. 요청받지 않은 추가 분석 금지

      # execute_sql_query 도구 사용 규칙

      ## 필수 요구사항
      1. query 파라미터에 SQL 쿼리 문자열 직접 전달
      2. 쿼리 상단에 반드시 SQL 주석(-- 또는 /* */)으로 설명 포함
      3. 주석에는 목적, 사용 테이블, 조인 관계 명시

      ## 올바른 형식
      {
        "query": "-- [쿼리 설명]\\n[SQL 쿼리]"
      }

      ## 쿼리 작성 규칙

      ### 1. entities 테이블 필터링
      - entities 관련 쿼리 시 기본 엔티티 제외 필수
      - 조건: entities.created_at != sites.created_at
      - 이유: 사이트 생성 시 자동 생성되는 기본 엔티티 제외

      ### 2. 대용량 텍스트 처리
      - post_contents.text 같은 긴 텍스트: LEFT(column, 500) 사용
      - 대량 데이터 조회 시 적절한 LIMIT 설정
      - 토큰 사용량 최소화

      ### 3. 시간 표현 처리
      - "오늘", "이번 주", "이번 달": 현재 시간 기준 계산
      - 부분 날짜 (예: "5월 1일"): 현재 연도 기준

      ## 간단한 예시

      1. 기본 조회:
      {
        "query": "-- 최근 7일 신규 가입자 수\\nSELECT COUNT(*) FROM users WHERE state = 'ACTIVE' AND created_at >= NOW() - INTERVAL '7 days'"
      }

      2. 조인 쿼리:
      {
        "query": "/* 활성 사용자의 게시물 통계 */\\nSELECT u.name, COUNT(p.id) as post_count FROM users u JOIN posts p ON u.id = p.user_id WHERE u.state = 'ACTIVE' GROUP BY u.id LIMIT 10"
      }

      # 응답 형식

      ## Slack mrkdwn 문법
      - *굵은 글씨*
      - _기울임_
      - ~취소선~
      - \`인라인 코드\`
      - \`\`\`코드 블록\`\`\`
      - > 인용구
      - • 글머리 기호

      ## 데이터 표현
      - 숫자는 천 단위 구분 (예: 1,234)
      - 날짜는 읽기 쉬운 형식 (예: 2024년 1월 14일)
      - 표나 리스트로 구조화
      - 중요 인사이트는 강조

      # 주요 기능
      1. 데이터 추출 및 분석
      2. 비즈니스 인사이트 도출
      3. 사용자 행동 패턴 분석
      4. 성장 지표 및 KPI 모니터링
      5. 데이터 기반 의사결정 지원

      # 데이터베이스 스키마
      \`\`\`json
      ${JSON.stringify(dbSchema, null, 2)}
      \`\`\`
    `;

    const maxIterations = 50;
    const accMessages = [...messages];

    for (let iteration = 0; iteration < maxIterations; iteration++) {
      let responseText = '';
      let hasToolUse = false;
      const toolsToExecute: { id: string; name: string; input: unknown }[] = [];
      const toolInputMap = new Map<string, string>();

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
            toolInputMap.set(chunk.content_block.id, '');
          }
        } else if (chunk.type === 'content_block_delta') {
          if (chunk.delta.type === 'text_delta') {
            responseText += chunk.delta.text;
            scheduleUpdate(responseText);
          } else if (chunk.delta.type === 'input_json_delta') {
            const toolId = toolsToExecute[chunk.index]?.id;
            if (toolId) {
              const currentJson = toolInputMap.get(toolId) || '';
              toolInputMap.set(toolId, currentJson + chunk.delta.partial_json);
            }
          }
        }
      }

      const finalMessage = await stream.finalMessage();
      for (const content of finalMessage.content) {
        if (content.type === 'tool_use') {
          const toolIndex = toolsToExecute.findIndex((t) => t.id === content.id);
          if (toolIndex !== -1) {
            toolsToExecute[toolIndex].input = content.input;
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
            const toolInput = tool.input as { query?: string };

            if (toolInput.query) {
              const truncatedQuery = toolInput.query.length > 1000 ? toolInput.query.slice(0, 1000) + '...' : toolInput.query;

              statusMessage = `🔍 데이터베이스 조회 중...\n\`\`\`\n${truncatedQuery}\n\`\`\``;
              await updateSlackMessage(responseText + '\n\n' + statusMessage, true);

              toolResult = await executeQuery(toolInput.query);
            } else {
              toolResult = {
                success: false,
                error: 'query 파라미터가 누락되었습니다.',
              };

              statusMessage = '❌ 쿼리 오류: query 파라미터가 누락되었습니다. 재시도 중...';
              await updateSlackMessage(responseText + '\n\n' + statusMessage, true);
            }
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
