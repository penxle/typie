import { UserState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { sql } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { dbr, DocumentCharacterCountChanges, Documents, Entities, Plans, Sites, Subscriptions, Users } from '#/db/index.ts';
import { builder } from '../builder.ts';

// 유료 구독 모집단 — TRIAL·MANUAL(LIFETIME)과 주기 없는 플랜은 매출·활성 집계에서 제외한다.
const paidSubscriptions = sql`
  paid_subscriptions AS (
    SELECT
      ${Subscriptions.id} AS id,
      ${Subscriptions.userId} AS user_id,
      ${Subscriptions.state} AS state,
      ${Subscriptions.startsAt} AS starts_at,
      ${Subscriptions.currentPeriodStartsAt} AS current_period_starts_at,
      ${Subscriptions.currentPeriodEndsAt} AS current_period_ends_at,
      ${Plans.availability} AS availability,
      CASE
        WHEN ${Plans.interval} = 'MONTHLY' THEN ${Plans.fee}
        WHEN ${Plans.interval} = 'YEARLY' THEN ${Plans.fee} / 12
        ELSE 0
      END AS monthly_fee
    FROM ${Subscriptions}
    INNER JOIN ${Plans} ON ${Subscriptions.planId} = ${Plans.id}
    WHERE ${Plans.availability} NOT IN ('TRIAL', 'MANUAL')
      AND ${Plans.interval} IN ('MONTHLY', 'YEARLY')
      AND ${Subscriptions.state} != 'EXPIRED'
  )
`;

// 권한 판정식을 재투영 날짜(date_series.date)에 그대로 옮긴 술어. 단순 기간 비교로 접으면
// 전환 유예가 미래 주기 종료까지 활성으로 남는다. 유예 마감은 판정식의 파생 규칙 그대로
// 채널 분기한다 — IAP 는 주기 종료 + 31일 백스톱, 그 외는 그날 이하 주기 컬럼 최대값 + 7일.
const entitledOnDate = sql`
  s.starts_at <= (date_series.date + interval '1 day')
  AND (
    s.state = 'ACTIVE'
    OR (s.state = 'WILL_EXPIRE' AND s.current_period_ends_at > date_series.date)
    OR (s.state = 'IN_GRACE_PERIOD' AND CASE
          WHEN s.availability = 'IN_APP_PURCHASE' THEN s.current_period_ends_at + interval '31 days'
          ELSE COALESCE(GREATEST(
            CASE WHEN s.current_period_starts_at <= date_series.date THEN s.current_period_starts_at END,
            CASE WHEN s.current_period_ends_at <= date_series.date THEN s.current_period_ends_at END
          ), s.current_period_starts_at) + interval '7 days'
        END > date_series.date)
    OR (s.state = 'WILL_ACTIVATE' AND s.starts_at <= date_series.date)
  )
`;

builder.queryField('stats', (t) =>
  t.field({
    type: 'JSON',
    resolve: async () => {
      const cacheKey = 'stats:v2';

      const cached = await redis.get(cacheKey);
      if (cached) {
        return JSON.parse(cached);
      }

      const current = dayjs();
      const now = current.toISOString();
      const thirtyDaysAgo = current.subtract(30, 'days').toISOString();
      const twentyFourHoursAgo = current.subtract(24, 'hours').toISOString();
      const fortyEightHoursAgo = current.subtract(48, 'hours').toISOString();

      // User metrics
      const getUsersTotal = () =>
        dbr.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          )
          SELECT 
            date_series.date::text as date,
            COALESCE(COUNT(${Users.id}), 0)::int as value
          FROM date_series
          LEFT JOIN ${Users} ON ${Users.createdAt} < (date_series.date + interval '1 day') 
            AND ${Users.state} = ${UserState.ACTIVE}
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getUsersNew = () =>
        dbr.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          current_period AS (
            SELECT COUNT(${Users.id})::int as count
            FROM ${Users}
            WHERE ${Users.createdAt} >= ${twentyFourHoursAgo}
              AND ${Users.createdAt} < ${now}
              AND ${Users.state} = ${UserState.ACTIVE}
          ),
          previous_period AS (
            SELECT COUNT(${Users.id})::int as count
            FROM ${Users}
            WHERE ${Users.createdAt} >= ${fortyEightHoursAgo}
              AND ${Users.createdAt} < ${twentyFourHoursAgo}
              AND ${Users.state} = ${UserState.ACTIVE}
          )
          SELECT 
            date_series.date::text as date,
            CASE 
              WHEN date_series.date = CURRENT_DATE - INTERVAL '1 day' THEN COALESCE((SELECT count FROM previous_period), 0)
              WHEN date_series.date = CURRENT_DATE THEN COALESCE((SELECT count FROM current_period), 0)
              ELSE COALESCE(COUNT(${Users.id}), 0)
            END::int as value
          FROM date_series
          LEFT JOIN ${Users} ON DATE(${Users.createdAt}) = date_series.date 
            AND ${Users.state} = ${UserState.ACTIVE}
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getUsersActive = () =>
        dbr.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          valid_user_activities AS (
            SELECT ${DocumentCharacterCountChanges.userId} AS user_id, ${DocumentCharacterCountChanges.bucket} AS bucket
            FROM ${DocumentCharacterCountChanges}
            INNER JOIN ${Documents} ON ${DocumentCharacterCountChanges.documentId} = ${Documents.id}
            INNER JOIN ${Entities} ON ${Documents.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Entities.createdAt} != ${Sites.createdAt}
          ),
          current_period AS (
            SELECT COUNT(DISTINCT user_id)::int as count
            FROM valid_user_activities
            WHERE bucket >= ${twentyFourHoursAgo}
              AND bucket < ${now}
          ),
          previous_period AS (
            SELECT COUNT(DISTINCT user_id)::int as count
            FROM valid_user_activities
            WHERE bucket >= ${fortyEightHoursAgo}
              AND bucket < ${twentyFourHoursAgo}
          )
          SELECT 
            date_series.date::text as date,
            CASE 
              WHEN date_series.date = CURRENT_DATE - INTERVAL '1 day' THEN COALESCE((SELECT count FROM previous_period), 0)
              WHEN date_series.date = CURRENT_DATE THEN COALESCE((SELECT count FROM current_period), 0)
              ELSE COALESCE(COUNT(DISTINCT vua.user_id), 0)
            END::int as value
          FROM date_series
          LEFT JOIN valid_user_activities vua ON DATE(vua.bucket) = date_series.date
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      // Subscription metrics
      // 상태 이력이 없으므로 과거 구간도 현재 상태의 재투영이다 — 이후의 환불·전이가 과거 관측치를 소급 변경한다.
      const getSubscriptionsRevenue = () =>
        dbr.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          ${paidSubscriptions}
          SELECT
            date_series.date::text as date,
            COALESCE(SUM(s.monthly_fee), 0)::int as value
          FROM date_series
          LEFT JOIN paid_subscriptions s ON ${entitledOnDate}
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getSubscriptionsActive = () =>
        dbr.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          ${paidSubscriptions}
          SELECT
            date_series.date::text as date,
            COALESCE(COUNT(DISTINCT s.user_id), 0)::int as value
          FROM date_series
          LEFT JOIN paid_subscriptions s ON ${entitledOnDate}
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      // Document metrics
      const getDocumentsTotal = () =>
        dbr.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          real_documents AS (
            SELECT DISTINCT ${Documents.id}, ${Documents.createdAt} AS created_at
            FROM ${Documents}
            INNER JOIN ${Entities} ON ${Documents.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Entities.createdAt} != ${Sites.createdAt}
          )
          SELECT 
            date_series.date::text as date,
            COALESCE(COUNT(rd.id), 0)::int as value
          FROM date_series
          LEFT JOIN real_documents rd ON rd.created_at < (date_series.date + interval '1 day')
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      // Character metrics
      const getCharactersInput = () =>
        dbr.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          valid_character_changes AS (
            SELECT ${DocumentCharacterCountChanges.bucket} AS bucket, ${DocumentCharacterCountChanges.additions} AS additions
            FROM ${DocumentCharacterCountChanges}
            INNER JOIN ${Documents} ON ${DocumentCharacterCountChanges.documentId} = ${Documents.id}
            INNER JOIN ${Entities} ON ${Documents.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Entities.createdAt} != ${Sites.createdAt}
          )
          SELECT
            date_series.date::text as date,
            COALESCE(SUM(vcc.additions), 0)::int as value
          FROM date_series
          LEFT JOIN valid_character_changes vcc ON vcc.bucket < (date_series.date + interval '1 day')
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getCharactersDaily = () =>
        dbr.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          valid_character_changes AS (
            SELECT ${DocumentCharacterCountChanges.bucket} AS bucket, ${DocumentCharacterCountChanges.additions} AS additions
            FROM ${DocumentCharacterCountChanges}
            INNER JOIN ${Documents} ON ${DocumentCharacterCountChanges.documentId} = ${Documents.id}
            INNER JOIN ${Entities} ON ${Documents.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Entities.createdAt} != ${Sites.createdAt}
          ),
          current_period AS (
            SELECT SUM(additions)::int as total
            FROM valid_character_changes
            WHERE bucket >= ${twentyFourHoursAgo}
              AND bucket < ${now}
          ),
          previous_period AS (
            SELECT SUM(additions)::int as total
            FROM valid_character_changes
            WHERE bucket >= ${fortyEightHoursAgo}
              AND bucket < ${twentyFourHoursAgo}
          )
          SELECT 
            date_series.date::text as date,
            CASE 
              WHEN date_series.date = CURRENT_DATE - INTERVAL '1 day' THEN COALESCE((SELECT total FROM previous_period), 0)
              WHEN date_series.date = CURRENT_DATE THEN COALESCE((SELECT total FROM current_period), 0)
              ELSE COALESCE(SUM(vcc.additions), 0)
            END::int as value
          FROM date_series
          LEFT JOIN valid_character_changes vcc ON DATE(vcc.bucket) = date_series.date
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      // System metrics
      const getSystemServiceDays = () =>
        dbr.execute(sql`
          WITH service_launch AS (
            SELECT MIN(${Users.createdAt})::date as launch_date
            FROM ${Users}
            WHERE ${Users.state} = ${UserState.ACTIVE}
          ),
          date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          )
          SELECT 
            date_series.date::text as date,
            (date_series.date - sl.launch_date + 1)::int as value
          FROM date_series
          CROSS JOIN service_launch sl
          ORDER BY date_series.date
        `);

      const [
        usersTotal,
        usersNew,
        usersActive,
        subscriptionsRevenue,
        subscriptionsActive,
        documentsTotal,
        charactersInput,
        charactersDaily,
        systemServiceDays,
      ] = await Promise.all([
        getUsersTotal(),
        getUsersNew(),
        getUsersActive(),
        getSubscriptionsRevenue(),
        getSubscriptionsActive(),
        getDocumentsTotal(),
        getCharactersInput(),
        getCharactersDaily(),
        getSystemServiceDays(),
      ]);

      const transformToData = (rows: Record<string, unknown>[]) => {
        const data = rows.map((row) => ({
          date: String(row.date),
          value: Number(row.value),
        }));

        return { data, current: data.at(-1)?.value ?? 0 };
      };

      const result = {
        // User metrics
        usersTotal: transformToData(usersTotal),
        usersNew: transformToData(usersNew),
        usersActive: transformToData(usersActive),

        // Subscription metrics
        subscriptionsRevenue: transformToData(subscriptionsRevenue),
        subscriptionsActive: transformToData(subscriptionsActive),

        // Document metrics
        documentsTotal: transformToData(documentsTotal),

        // Character metrics
        charactersInput: transformToData(charactersInput),
        charactersDaily: transformToData(charactersDaily),

        // System metrics
        systemServiceDays: transformToData(systemServiceDays),
      };

      await redis.setex(cacheKey, 3600, JSON.stringify(result));

      return result;
    },
  }),
);

builder.queryField('activeWritersCount', (t) =>
  t.field({
    type: 'Int',
    resolve: async () => {
      const sevenDaysAgo = Date.now() - 7 * 24 * 60 * 60 * 1000;

      await redis.zremrangebyscore('writers:active', '-inf', sevenDaysAgo);

      const count = await redis.zcard('writers:active');
      return count;
    },
  }),
);
