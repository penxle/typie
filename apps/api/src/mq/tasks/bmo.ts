import '@typie/lib/dayjs';

import { createSdkMcpServer, query, tool } from '@anthropic-ai/claude-agent-sdk';
import { WebClient } from '@slack/web-api';
import dayjs from 'dayjs';
import dedent from 'dedent';
import postgres from 'postgres';
import { remark } from 'remark';
import remarkGfm from 'remark-gfm';
import { match } from 'ts-pattern';
import { z } from 'zod';
import { redis } from '@/cache';
import { env } from '@/env';
import { defineJob } from '../types';
import type { Options } from '@anthropic-ai/claude-agent-sdk';
import type { Nodes } from 'mdast';

type SlackAppMentionEventPayload = {
  user: string;
  text: string;
  ts: string;
  thread_ts?: string;
  channel: string;
  event_ts: string;
};

const sql = postgres(env.DATABASE_URL, {
  prepare: false,
});

const slack = new WebClient(env.SLACK_BOT_TOKEN);

const SESSION_KEY_PREFIX = 'bmo:session:';
const SESSION_TTL = 60 * 60 * 24 * 7;

const executeQuery = async (query: string) => {
  try {
    return await sql.begin('READ ONLY', async (sql) => {
      const result = await sql.unsafe(query);

      return {
        success: true,
        count: result.length,
        rows: [...result],
      };
    });
  } catch (err) {
    return {
      success: false,
      error: err instanceof Error ? err.message : String(err),
    };
  }
};

let schema: unknown | null = null;

// spell-checker:disable
const getSchemaQuery = dedent`
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
    await sql.begin('READ ONLY', async (sql) => {
      const result = await sql.unsafe(getSchemaQuery);
      schema = result[0].schema;
    });
  }

  return schema;
};

export const ProcessBmoMentionJob = defineJob('bmo:process-mention', async (event: SlackAppMentionEventPayload) => {
  let messageTs: string | undefined;
  type Entry =
    | { type: 'status'; text: string }
    | { type: 'thinking' }
    | { type: 'text'; text: string }
    | { type: 'query'; description: string; status: 'running' | 'completed' | 'failed' }
    | { type: 'error'; text: string };
  const entries: Entry[] = [{ type: 'status', text: '⏳ _준비 중..._' }];
  let latestAssistantText = '';
  let currentTurnTextEntry: Extract<Entry, { type: 'text' }> | null = null;

  const toSlackMrkdwn = (markdown: string) => {
    const tree = remark().use(remarkGfm).parse(markdown);

    const renderInline = (nodes: Nodes[]): string => {
      const parts: { text: string; formatted: boolean }[] = nodes.map((node) =>
        match(node)
          .with({ type: 'text' }, (n) => ({ text: n.value, formatted: false }))
          .with({ type: 'strong' }, (n) => ({ text: `*${renderInline(n.children)}*`, formatted: true }))
          .with({ type: 'emphasis' }, (n) => ({ text: `_${renderInline(n.children)}_`, formatted: true }))
          .with({ type: 'delete' }, (n) => ({ text: `~${renderInline(n.children)}~`, formatted: true }))
          .with({ type: 'inlineCode' }, (n) => ({ text: `\`${n.value}\``, formatted: false }))
          .with({ type: 'link' }, (n) => ({ text: `<${n.url}|${renderInline(n.children)}>`, formatted: false }))
          .with({ type: 'image' }, (n) => ({ text: `<${n.url}>`, formatted: false }))
          .with({ type: 'break' }, () => ({ text: '\n', formatted: false }))
          .otherwise((n) => ({ text: 'children' in n ? renderInline(n.children as Nodes[]) : '', formatted: false })),
      );

      let result = '';
      for (let i = 0; i < parts.length; i++) {
        const { text, formatted } = parts[i];
        if (formatted && result.length > 0 && !/[\s([]$/.test(result)) {
          result += ' ';
        }
        result += text;
        if (formatted && i < parts.length - 1) {
          const next = parts[i + 1];
          if (!/^[\s)\].,!?;:、。]/.test(next.text)) {
            result += ' ';
          }
        }
      }
      return result;
    };

    const renderBlock = (nodes: Nodes[], indent = ''): string => {
      return nodes
        .map((node) =>
          match(node)
            .with({ type: 'heading' }, (n) => `${indent}*${renderInline(n.children)}*`)
            .with({ type: 'paragraph' }, (n) => `${indent}${renderInline(n.children)}`)
            .with({ type: 'code' }, (n) => `${indent}\`\`\`\n${n.value}\n\`\`\``)
            .with({ type: 'blockquote' }, (n) =>
              renderBlock(n.children as Nodes[])
                .split('\n')
                .map((line) => `${indent}> ${line}`)
                .join('\n'),
            )
            .with({ type: 'list' }, (n) => {
              const nestedIndent = indent + '  ';
              return (n.children as Nodes[])
                .map((item, i) => {
                  if (item.type !== 'listItem') return '';
                  const prefix = n.ordered ? `${(n.start ?? 1) + i}.` : '•';
                  const checked = item.checked;
                  const checkbox = checked === true ? '☑ ' : checked === false ? '☐ ' : '';
                  const content = (item.children as Nodes[])
                    .map((child) =>
                      match(child)
                        .with({ type: 'paragraph' }, (p) => renderInline(p.children))
                        .otherwise((c) => renderBlock([c], nestedIndent)),
                    )
                    .join('\n');
                  return `${indent}${prefix} ${checkbox}${content}`;
                })
                .join('\n');
            })
            .with({ type: 'thematicBreak' }, () => `${indent}──────────`)
            .with({ type: 'html' }, (n) => `${indent}${n.value}`)
            .with({ type: 'table' }, (n) => renderTable(n))
            .otherwise((n) => ('children' in n ? renderBlock(n.children as Nodes[], indent) : '')),
        )
        .join('\n\n');
    };

    const renderTable = (node: Nodes): string => {
      if (node.type !== 'table' || !('children' in node)) return '';
      return (node.children as Nodes[])
        .map((row, i) =>
          match(row)
            .with({ type: 'tableRow' }, (r) => {
              const cells = (r.children as Nodes[]).map((cell) => ('children' in cell ? renderInline(cell.children as Nodes[]) : ''));
              const line = cells.join(' | ');
              return i === 0 ? `*${line}*` : line;
            })
            .otherwise(() => ''),
        )
        .join('\n');
    };

    return renderBlock(tree.children as Nodes[]);
  };

  const buildAttachments = () => {
    return entries.map((entry) =>
      match(entry)
        .with({ type: 'status' }, (e) => ({ color: '#808080', text: e.text, mrkdwn_in: ['text' as const] }))
        .with({ type: 'thinking' }, () => ({ color: '#808080', text: '💭 _생각 중..._', mrkdwn_in: ['text' as const] }))
        .with({ type: 'text' }, (e) => ({ color: '#3498db', text: toSlackMrkdwn(e.text), mrkdwn_in: ['text' as const] }))
        .with({ type: 'query', status: 'completed' }, (e) => ({
          color: '#2ecc71',
          text: `✅ ${e.description}`,
          mrkdwn_in: ['text' as const],
        }))
        .with({ type: 'query', status: 'running' }, (e) => ({
          color: '#f39c12',
          text: `🔍 _${e.description}..._`,
          mrkdwn_in: ['text' as const],
        }))
        .with({ type: 'query', status: 'failed' }, (e) => ({
          color: '#e74c3c',
          text: `❌ ${e.description}`,
          mrkdwn_in: ['text' as const],
        }))
        .with({ type: 'error' }, (e) => ({ color: '#e74c3c', text: `❌ ${e.text}`, mrkdwn_in: ['text' as const] }))
        .exhaustive(),
    );
  };

  const flushSlackMessage = async () => {
    if (!messageTs) return;

    try {
      await slack.chat.update({
        channel: event.channel,
        ts: messageTs,
        text: '',
        attachments: buildAttachments(),
      });
    } catch (err) {
      console.error('[bmo] chat.update error:', err);
    }
  };

  try {
    const text = event.text.replaceAll(/<@[^>]+>/g, '').trim() || '안녕하세요';

    const initialMessage = await slack.chat.postMessage({
      channel: event.channel,
      thread_ts: event.thread_ts || event.ts,
      text: '',
      attachments: buildAttachments(),
      reply_broadcast: !event.thread_ts,
    });

    messageTs = initialMessage.ts;

    const threadKey = event.thread_ts || event.ts;
    const existingSessionId = await redis.get(`${SESSION_KEY_PREFIX}${threadKey}`);

    const bmoServer = createSdkMcpServer({
      name: 'bmo',
      tools: [
        tool(
          'execute_sql_query',
          'PostgreSQL 데이터베이스에서 읽기 전용 트랜잭션으로 쿼리를 실행합니다. SELECT, WITH, SHOW, EXPLAIN 등 읽기 작업만 가능합니다.',
          {
            description: z.string().describe('쿼리의 목적을 간단히 설명하는 문장'),
            query: z.string().describe('SQL 쿼리 문자열'),
          },
          async (args) => {
            entries.push({ type: 'query', description: args.description, status: 'running' });
            await flushSlackMessage();

            const result = await executeQuery(args.query);
            for (let i = entries.length - 1; i >= 0; i--) {
              const entry = entries[i];
              if (entry.type === 'query' && entry.status === 'running') {
                entry.status = result.success ? 'completed' : 'failed';
                break;
              }
            }
            await flushSlackMessage();

            return {
              content: [{ type: 'text' as const, text: JSON.stringify(result) }],
            };
          },
        ),
      ],
    });

    const queryOptions: Options = {
      mcpServers: { bmo: bmoServer },
      allowedTools: ['mcp__bmo__*'],
      model: 'claude-sonnet-4-5-20250929',
      maxThinkingTokens: 20_000,
      includePartialMessages: true,
      permissionMode: 'dontAsk',
      betas: ['context-1m-2025-08-07'],
      env: { ...process.env, ANTHROPIC_API_KEY: env.ANTHROPIC_API_KEY, CLAUDE_CODE_STREAM_CLOSE_TIMEOUT: '300' },
      stderr: (data) => console.error('[bmo:claude]', data),
    };

    if (existingSessionId) {
      queryOptions.resume = existingSessionId;
    } else {
      const dbSchema = await getDatabaseSchema();

      queryOptions.systemPrompt = dedent`
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

        # 응답 형식

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
    }

    async function* generateMessages() {
      yield {
        type: 'user' as const,
        session_id: '',
        parent_tool_use_id: null,
        message: {
          role: 'user' as const,
          content: text,
        },
      };
    }

    for await (const message of query({
      prompt: generateMessages(),
      options: queryOptions,
    })) {
      if ('session_id' in message && message.session_id) {
        await redis.set(`${SESSION_KEY_PREFIX}${threadKey}`, message.session_id as string, 'EX', SESSION_TTL);
      }

      if (message.type === 'stream_event') {
        const evt = message.event;
        if (evt.type === 'content_block_start' && evt.content_block?.type === 'thinking') {
          entries.push({ type: 'thinking' });
          currentTurnTextEntry = null;
          latestAssistantText = '';
          await flushSlackMessage();
        }
      } else if (message.type === 'assistant') {
        if (message.message?.content) {
          for (const block of message.message.content) {
            if ('text' in block && typeof block.text === 'string') {
              latestAssistantText = block.text;
            }
          }
          const hasToolUse = message.message.content.some((b) => b.type === 'tool_use');
          if (hasToolUse && latestAssistantText) {
            if (currentTurnTextEntry) {
              currentTurnTextEntry.text = latestAssistantText;
            } else {
              currentTurnTextEntry = { type: 'text' as const, text: latestAssistantText };
              entries.push(currentTurnTextEntry);
            }
            await flushSlackMessage();
          }
        }
      } else if (message.type === 'result') {
        if (latestAssistantText) {
          if (currentTurnTextEntry) {
            currentTurnTextEntry.text = latestAssistantText;
          } else {
            entries.push({ type: 'text', text: latestAssistantText });
          }
        }
        if (!entries.some((e) => e.type === 'text')) {
          entries.push({ type: 'error', text: '응답을 생성할 수 없었어요.' });
        }
        await flushSlackMessage();
      }
    }
  } catch (err) {
    console.error('[bmo] error:', err);
    entries.push({ type: 'error', text: `오류가 발생했어요.\n\`\`\`${err instanceof Error ? err.message : String(err)}\`\`\`` });
    await flushSlackMessage();

    throw err;
  }
});
