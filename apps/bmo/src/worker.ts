import { mkdirSync } from 'node:fs';
import { createSdkMcpServer, query, tool } from '@anthropic-ai/claude-agent-sdk';
import { WebClient } from '@slack/web-api';
import dedent from 'dedent';
import { match } from 'ts-pattern';
import { z } from 'zod';
import { loadEnv } from './env.ts';
import { getSession, setSession } from './session.ts';
import { toSlackMrkdwn } from './slack-mrkdwn.ts';
import type { Options } from '@anthropic-ai/claude-agent-sdk';
import type { BetaContentBlock } from '@anthropic-ai/sdk/resources/beta';
import type { SlackAppMentionEvent } from './slack-types.ts';

mkdirSync('/tmp/.claude/debug', { recursive: true });

let dbSchema: unknown | null = null;

const executeQuery = async (apiBaseUrl: string, apiSecret: string, sqlQuery: string) => {
  const res = await fetch(`${apiBaseUrl}/bmo/query`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${apiSecret}` },
    body: JSON.stringify({ query: sqlQuery }),
  });
  const raw = await res.text();
  const text = raw.trim();
  if (!res.ok) {
    return { success: false as const, error: `API error ${res.status}: ${text}` };
  }
  return JSON.parse(text) as { success: boolean; count?: number; rows?: unknown[]; error?: string };
};

const getDatabaseSchema = async (apiBaseUrl: string, apiSecret: string) => {
  if (!dbSchema) {
    const res = await fetch(`${apiBaseUrl}/bmo/schema`, {
      headers: { Authorization: `Bearer ${apiSecret}` },
    });
    if (!res.ok) {
      throw new Error(`Schema API error ${res.status}: ${await res.text()}`);
    }
    dbSchema = await res.json();
  }
  return dbSchema;
};

const formatCurrentTime = () => {
  return new Intl.DateTimeFormat('ko-KR', {
    timeZone: 'Asia/Seoul',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    weekday: 'long',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  }).format(new Date());
};

export const handler = async (event: SlackAppMentionEvent) => {
  const env = await loadEnv();
  const slack = new WebClient(env.SLACK_BOT_TOKEN);

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
    const existingSessionId = await getSession(threadKey);

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

            const result = await executeQuery(env.API_BASE_URL, env.API_KEY, args.query);
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
      model: 'claude-sonnet-4-6',
      thinking: { type: 'adaptive' },
      effort: 'high',
      includePartialMessages: true,
      permissionMode: 'dontAsk',
      betas: ['context-1m-2025-08-07'],
      env: { ...process.env, ANTHROPIC_API_KEY: env.ANTHROPIC_API_KEY, CLAUDE_CODE_STREAM_CLOSE_TIMEOUT: '300' },
      stderr: (data) => console.error('[bmo:claude]', data),
    };

    if (existingSessionId) {
      queryOptions.resume = existingSessionId;
    } else {
      const schema = await getDatabaseSchema(env.API_BASE_URL, env.API_KEY);

      queryOptions.systemPrompt = dedent`
        # 시스템 정보
        현재 시간: ${formatCurrentTime()} (Asia/Seoul)

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
        ${JSON.stringify(schema, null, 2)}
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
        await setSession(threadKey, message.session_id as string);
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
          const hasToolUse = message.message.content.some((b: BetaContentBlock) => b.type === 'tool_use');
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
};
