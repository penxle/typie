import '@typie/lib/dayjs';

import { WebClient } from '@slack/web-api';
import dayjs from 'dayjs';
import dedent from 'dedent';
import { Pool } from 'pg';
import { env } from '@/env';
import { enqueueJob } from '@/mq';
import { defineCron, defineJob } from '../types';

type SlackAppMentionEventPayload = {
  user: string;
  text: string;
  ts: string;
  thread_ts?: string;
  channel: string;
  event_ts: string;
};

const slack = new WebClient(env.GIR_SLACK_BOT_TOKEN);

const pool = new Pool({
  connectionString: env.DATABASE_URL,
  ssl: { rejectUnauthorized: false },
  max: 5,
  idleTimeoutMillis: 10 * 60 * 1000,
  statement_timeout: 60_000,
});

pool.on('connect', (client) => {
  client.query("SET TIME ZONE 'Asia/Seoul'");
});

const executeQuery = async (query: string) => {
  const client = await pool.connect();
  try {
    await client.query('BEGIN READ ONLY');
    const result = await client.query(query);
    await client.query('COMMIT');

    return {
      success: true,
      count: result.rows.length,
      rows: [...result.rows],
    };
  } catch (err) {
    await client.query('ROLLBACK');
    return {
      success: false,
      error: err instanceof Error ? err.message : String(err),
    };
  } finally {
    client.release();
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
    const client = await pool.connect();
    try {
      await client.query('BEGIN READ ONLY');
      const result = await client.query(getSchemaQuery);
      await client.query('COMMIT');
      const [row] = result.rows;
      schema = row?.schema || { tables: [], enums: [] };
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  }

  return schema;
};

const generateDailyReport = async (channel: string) => {
  let messageTs: string | undefined;

  try {
    // 시작 메시지
    const initialMessage = await slack.chat.postMessage({
      channel,
      text: '🐶 GIR 일일 리포트 생성을 시작합니다...',
      mrkdwn: true,
    });
    messageTs = initialMessage.ts;

    const dbSchema = await getDatabaseSchema();
    const today = dayjs.kst();
    const todayFormatted = today.format('YYYY년 MM월 DD일 dddd');

    const coreMetrics = await Promise.all([
      executeQuery(`
        -- 최근 24시간 신규 가입자
        SELECT COUNT(*) as new_users_24h
        FROM users
        WHERE created_at >= NOW() - INTERVAL '24 hours'
          AND state = 'ACTIVE'
      `),

      executeQuery(`
        -- 최근 24시간 신규 구독자
        SELECT COUNT(*) as new_subscribers_24h
        FROM subscriptions
        WHERE created_at >= NOW() - INTERVAL '24 hours'
          AND state = 'ACTIVE'
      `),

      executeQuery(`
        -- 전체 활성 가입자
        SELECT COUNT(*) as total_users
        FROM users
        WHERE state = 'ACTIVE'
      `),

      executeQuery(`
        -- 전체 활성 구독자
        SELECT COUNT(*) as total_subscribers
        FROM subscriptions
        WHERE state = 'ACTIVE'
      `),
    ]);

    const coreMetricsData = {
      newUsers24h: coreMetrics[0].success ? coreMetrics[0].rows?.[0]?.new_users_24h || 0 : 0,
      newSubscribers24h: coreMetrics[1].success ? coreMetrics[1].rows?.[0]?.new_subscribers_24h || 0 : 0,
      totalUsers: coreMetrics[2].success ? coreMetrics[2].rows?.[0]?.total_users || 0 : 0,
      totalSubscribers: coreMetrics[3].success ? coreMetrics[3].rows?.[0]?.total_subscribers || 0 : 0,
    };

    const tableNames = (dbSchema as { tables: { table_name: string }[] }).tables.map((table) => table.table_name);
    const randomTableName = tableNames[Math.floor(Math.random() * tableNames.length)];

    const reportMessage = dedent(
      `
        🐶 *GIR 일일 리포트* - ${todayFormatted}
        <@${env.BMO_SLACK_BOT_ID}> 오늘은 \`${randomTableName}\` 테이블이 신경쓰여요. 관련해서 흥미로운 인사이트를 아무거나 하나 알려주세요.

        📊 참고로 오늘의 핵심 지표는...
        • 신규 가입자 (24시간): *${coreMetricsData.newUsers24h}명*
        • 신규 구독자 (24시간): *${coreMetricsData.newSubscribers24h}명*
        • 전체 가입자: *${coreMetricsData.totalUsers.toLocaleString('ko-KR')}명*
        • 전체 구독자: *${coreMetricsData.totalSubscribers.toLocaleString('ko-KR')}명*
      `,
    );

    if (messageTs) {
      await slack.chat.update({
        channel,
        ts: messageTs,
        text: reportMessage,
      });
    }
  } catch (err) {
    const errorMessage = `⚠️ 일일 리포트 생성 중 오류가 발생했습니다.\n\`\`\`${err instanceof Error ? err.message : String(err)}\`\`\``;

    if (messageTs) {
      await slack.chat.update({
        channel,
        ts: messageTs,
        text: errorMessage,
      });
    } else {
      await slack.chat.postMessage({
        channel,
        text: errorMessage,
        mrkdwn: true,
      });
    }

    throw err;
  }
};

export const DailyAmazingFactJob = defineJob('gir:daily-amazing-fact', async () => {
  await generateDailyReport(env.GIR_DAILY_CHANNEL);
});

export const ProcessGirMentionJob = defineJob('gir:process-mention', async (event: SlackAppMentionEventPayload) => {
  await slack.reactions.add({
    channel: event.channel,
    timestamp: event.ts,
    name: 'dog',
  });

  await generateDailyReport(event.channel);
});

export const GirCron = defineCron(
  'gir:daily-amazing-fact:cron',
  '50 1 * * 1-5', // 평일(월-금) 한국시간 오전 10시 50분 (UTC 01:50)
  async () => {
    await enqueueJob('gir:daily-amazing-fact', {});
  },
);
