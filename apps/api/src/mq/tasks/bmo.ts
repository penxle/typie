import '@typie/lib/dayjs';

import { Anthropic } from '@anthropic-ai/sdk';
import { GetObjectCommand, PutObjectCommand } from '@aws-sdk/client-s3';
import { getSignedUrl } from '@aws-sdk/s3-request-presigner';
import { WebClient } from '@slack/web-api';
import dayjs from 'dayjs';
import dedent from 'dedent';
import postgres from 'postgres';
import { env } from '@/env';
import * as aws from '@/external/aws';
import { generateChart } from '@/utils/chart-generation';
import { defineJob } from '../types';
import type { ChartData } from '@/utils/chart-generation';

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
  connection: {
    statement_timeout: 600_000,
    lock_timeout: 600_000,
  },
});

const anthropic = new Anthropic({ apiKey: env.ANTHROPIC_API_KEY });
const slack = new WebClient(env.SLACK_BOT_TOKEN);

const executeQuery = async (query: string) => {
  await sql.begin('READ ONLY', async (sql) => {
    try {
      const result = await sql.unsafe(query);

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
  });
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
    await sql.begin('READ ONLY', async (sql) => {
      const result = await sql.unsafe(getSchemaQuery);
      schema = result[0].schema;
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
      {
        name: 'upload_to_s3',
        description:
          'S3 버킷에 데이터를 업로드하고 다운로드 URL을 생성합니다. JSON, CSV, 텍스트 등 다양한 형식의 데이터를 업로드할 수 있습니다.',
        input_schema: {
          type: 'object',
          properties: {
            filename: {
              type: 'string',
              description: '업로드할 파일 이름 (예: report.json, data.csv)',
            },
            content: {
              type: 'string',
              description: '업로드할 파일 내용',
            },
            contentType: {
              type: 'string',
              description: 'MIME 타입 (예: application/json, text/csv, text/plain)',
              default: 'text/plain',
            },
          },
          required: ['filename', 'content'],
        },
      },
      {
        name: 'create_chart',
        description: '데이터를 시각화하여 차트 이미지를 생성하고 슬랙에 업로드합니다. 막대 차트, 선 차트, 원 차트를 지원합니다.',
        input_schema: {
          type: 'object',
          properties: {
            title: {
              type: 'string',
              description: '차트 제목',
            },
            type: {
              type: 'string',
              enum: ['bar', 'line', 'pie'],
              description: '차트 타입: bar (막대), line (선), pie (원)',
            },
            data: {
              type: 'object',
              description: '차트 데이터. 모든 차트: { labels: string[], datasets: [{ label: string, data: number[] }] }',
            },
          },
          required: ['title', 'type', 'data'],
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
      - 목적: PostgreSQL 데이터베이스 쿼리를 통한 데이터 분석 및 인사이트, 차트 제공
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
      6. 데이터 시각화 및 차트 생성
      7. 분석 결과 파일 저장 및 공유

      # 도구 사용 가이드

      ## upload_to_s3 도구
      - 용도: 대용량 데이터나 상세 분석 결과를 파일로 저장하고 공유
      - 사용 시기:
        * 쿼리 결과가 너무 길어서 슬랙 메시지로 표시하기 어려울 때
        * CSV, JSON 형식의 원본 데이터를 공유해야 할 때
        * 정기 리포트나 백업 데이터를 보관할 때
        * 여러 사람과 데이터를 공유해야 할 때
      - 특징: 7일간 유효한 다운로드 링크 제공

      ## create_chart 도구
      - 용도: 데이터를 시각적으로 표현하여 인사이트 전달
      - 사용 시기:
        * 추세나 패턴을 한눈에 보여주고 싶을 때
        * 여러 항목의 비교가 필요할 때
        * 비율이나 구성을 표현할 때
        * 시계열 데이터의 변화를 보여줄 때
      - 지원 차트:
        * bar: 카테고리별 비교 (예: 월별 가입자 수)
        * line: 시간에 따른 변화 (예: 일별 활성 사용자)
        * pie: 구성 비율 (예: 사용자 유입 경로 비율)
      - 특징: 슬랙 스레드에 이미지로 바로 표시

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
        model: 'claude-sonnet-4-20250514',
        max_tokens: 64_000,
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
        if (content.type === 'text') {
          responseText = content.text;
        } else if (content.type === 'tool_use') {
          const toolIndex = toolsToExecute.findIndex((t) => t.id === content.id);
          if (toolIndex !== -1) {
            toolsToExecute[toolIndex].input = content.input;
          }
        }
      }

      if (updateTimer) {
        clearTimeout(updateTimer);
        updateTimer = null;
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
          } else if (tool.name === 'upload_to_s3') {
            const toolInput = tool.input as { filename?: string; content?: string; contentType?: string };

            if (toolInput.filename && toolInput.content) {
              try {
                statusMessage = `📤 S3에 파일 업로드 중: ${toolInput.filename}`;
                await updateSlackMessage(responseText + '\n\n' + statusMessage, true);

                const key = `bmo/${aws.createFragmentedS3ObjectKey()}_${toolInput.filename}`;
                const contentType = toolInput.contentType || 'text/plain';

                await aws.s3.send(
                  new PutObjectCommand({
                    Bucket: 'typie-misc',
                    Key: key,
                    Body: toolInput.content,
                    ContentType: contentType,
                  }),
                );

                const downloadUrl = await getSignedUrl(
                  aws.s3,
                  new GetObjectCommand({
                    Bucket: 'typie-misc',
                    Key: key,
                  }),
                  { expiresIn: 7 * 24 * 60 * 60 },
                );

                toolResult = {
                  success: true,
                  downloadUrl,
                  size: Buffer.byteLength(toolInput.content),
                  expiresAt: dayjs.kst().add(7, 'days').format('YYYY-MM-DD HH:mm:ss'),
                };
              } catch (err) {
                toolResult = {
                  success: false,
                  error: err instanceof Error ? err.message : String(err),
                };
              }
            } else {
              toolResult = {
                success: false,
                error: 'filename과 content 파라미터가 필요합니다.',
              };

              statusMessage = '❌ 업로드 오류: 필수 파라미터가 누락되었습니다.';
              await updateSlackMessage(responseText + '\n\n' + statusMessage, true);
            }
          } else if (tool.name === 'create_chart') {
            const toolInput = tool.input as {
              title?: string;
              type?: 'bar' | 'line' | 'pie';
              data?: unknown;
            };

            if (toolInput.title && toolInput.type && toolInput.data) {
              try {
                statusMessage = `📊 차트 생성 중: ${toolInput.title}`;
                await updateSlackMessage(responseText + '\n\n' + statusMessage, true);

                const chartBuffer = await generateChart(toolInput.title, toolInput.type, toolInput.data as ChartData);

                const uploadResult = await slack.files.uploadV2({
                  file: chartBuffer,
                  filename: 'chart.png',
                  title: toolInput.title,
                });

                if (uploadResult.ok) {
                  const filesResult = uploadResult as {
                    ok: boolean;
                    files?: { ok: boolean; files: { id: string; name: string; permalink: string }[] }[];
                    error?: string;
                  };

                  if (filesResult.files?.[0]) {
                    await slack.chat.postMessage({
                      channel: event.channel,
                      thread_ts: event.thread_ts || event.ts,
                      text: `📊 차트를 생성했습니다: ${filesResult.files[0].files[0].permalink}`,
                      reply_broadcast: !event.thread_ts,
                    });

                    toolResult = {
                      success: true,
                      fileId: filesResult.files[0].files[0].id,
                    };
                  } else {
                    toolResult = {
                      success: false,
                      error: '파일 업로드는 성공했으나 파일 정보를 가져올 수 없습니다.',
                    };
                  }
                } else {
                  toolResult = {
                    success: false,
                    error: uploadResult.error || '차트 업로드에 실패했습니다.',
                  };
                }
              } catch (err) {
                toolResult = {
                  success: false,
                  error: err instanceof Error ? err.message : String(err),
                };
              }
            } else {
              toolResult = {
                success: false,
                error: 'title, type, data 파라미터가 필요합니다.',
              };

              statusMessage = '❌ 차트 생성 오류: 필수 파라미터가 누락되었습니다.';
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

        let finalText = '';
        for (const content of finalMessage.content) {
          if (content.type === 'text') {
            finalText = content.text;
          }
        }

        if (finalText) {
          await updateSlackMessage(finalText, true);
        }

        accMessages.push(
          {
            role: 'assistant' as const,
            content: finalMessage.content,
          },
          ...toolResults,
        );
      } else {
        if (responseText) {
          await updateSlackMessage(responseText, true);
        }
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
