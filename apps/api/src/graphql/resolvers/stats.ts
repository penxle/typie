import { UserState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { sql } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { dbr, DocumentCharacterCountChanges, Documents, Entities, Plans, Sites, Subscriptions, Users } from '#/db/index.ts';
import { SYSTEM_USER_ID } from '#/utils/system-actor.ts';
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
      ${Subscriptions.createdAt} AS created_at,
      ${Plans.availability} AS availability,
      CASE
        WHEN ${Plans.interval} = 'MONTHLY' THEN ${Plans.fee}::numeric
        WHEN ${Plans.interval} = 'YEARLY' THEN ${Plans.fee} / 12.0
        ELSE 0
      END AS monthly_fee
    FROM ${Subscriptions}
    INNER JOIN ${Plans} ON ${Subscriptions.planId} = ${Plans.id}
    WHERE ${Plans.availability} NOT IN ('TRIAL', 'MANUAL')
      AND ${Plans.interval} IN ('MONTHLY', 'YEARLY')
      AND ${Subscriptions.state} != 'EXPIRED'
  )
`;

// 각 날짜의 경계·판정 인스턴트 = 그날 KST 24:00. date 연산만 거치므로 세션 타임존과 무관하다.
const kstDayEnd = sql`((date_series.date + 1)::timestamp AT TIME ZONE 'Asia/Seoul')`;

// 권한 판정식(isSubscriptionEntitled·deriveGraceDeadline)에 now := 그날 KST 24:00 을 대입한 술어.
// 단순 기간 비교로 접으면 전환 유예가 미래 주기 종료까지 활성으로 남는다. 유예 마감은 판정식의 파생 규칙 그대로
// 채널 분기한다 — IAP 는 주기 종료 + 31일 백스톱, 그 외는 판정 시점 이하 주기 컬럼 최대값(없으면 시작 컬럼) + 7일.
const entitledOnDate = sql`
  s.starts_at <= ${kstDayEnd}
  AND (
    s.state = 'ACTIVE'
    OR (s.state = 'WILL_EXPIRE' AND s.current_period_ends_at > ${kstDayEnd})
    OR (s.state = 'IN_GRACE_PERIOD' AND CASE
          WHEN s.availability = 'IN_APP_PURCHASE' THEN s.current_period_ends_at + interval '31 days'
          ELSE COALESCE(GREATEST(
            CASE WHEN s.current_period_starts_at <= ${kstDayEnd} THEN s.current_period_starts_at END,
            CASE WHEN s.current_period_ends_at <= ${kstDayEnd} THEN s.current_period_ends_at END
          ), s.current_period_starts_at) + interval '7 days'
        END > ${kstDayEnd})
    OR (s.state = 'WILL_ACTIVATE' AND s.starts_at <= ${kstDayEnd})
  )
`;

// 가입 트랜잭션에서 복사된 템플릿 문서(사이트와 같은 tx 라 now() 동일)와 시스템 액터의 정리 기여는 사용자 활동이 아니다.
const validChanges = sql`
  valid_changes AS (
    SELECT
      ${DocumentCharacterCountChanges.userId} AS user_id,
      ${DocumentCharacterCountChanges.bucket} AS bucket,
      ${DocumentCharacterCountChanges.additions} AS additions
    FROM ${DocumentCharacterCountChanges}
    INNER JOIN ${Documents} ON ${DocumentCharacterCountChanges.documentId} = ${Documents.id}
    INNER JOIN ${Entities} ON ${Documents.entityId} = ${Entities.id}
    INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
    WHERE ${Entities.createdAt} != ${Sites.createdAt}
      AND ${DocumentCharacterCountChanges.userId} != ${SYSTEM_USER_ID}
      AND (${DocumentCharacterCountChanges.additions} > 0 OR ${DocumentCharacterCountChanges.deletions} > 0)
  )
`;

builder.queryField('stats', (t) =>
  t.field({
    type: 'JSON',
    resolve: async () => {
      const cacheKey = 'stats:v3';

      const cached = await redis.get(cacheKey);
      if (cached) {
        return JSON.parse(cached);
      }

      // 상태 이력이 없으므로 모든 지표의 과거 구간은 현재 상태의 재투영이다 — 탈퇴·만료·전이·삭제가 과거 관측치를 소급 변경한다.
      const current = dayjs();
      const now = current.toISOString();
      const twentyFourHoursAgo = current.subtract(24, 'hours').toISOString();
      const seriesEnd = current.kst().format('YYYY-MM-DD');
      const seriesStart = current.kst().subtract(30, 'days').format('YYYY-MM-DD');

      const dateSeries = sql`
        date_series AS (
          SELECT generate_series(${seriesStart}::date, ${seriesEnd}::date, interval '1 day')::date AS date
        )
      `;

      // User metrics
      const getUsersTotal = () =>
        dbr.execute(sql`
          WITH ${dateSeries}
          SELECT
            date_series.date::text as date,
            COUNT(${Users.id})::int as value
          FROM date_series
          LEFT JOIN ${Users} ON ${Users.createdAt} < ${kstDayEnd}
            AND ${Users.state} = ${UserState.ACTIVE}
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getUsersNew = () =>
        dbr.execute(sql`
          WITH ${dateSeries},
          current_window AS (
            SELECT COUNT(${Users.id})::int as count
            FROM ${Users}
            WHERE ${Users.createdAt} >= ${twentyFourHoursAgo}
              AND ${Users.createdAt} < ${now}
              AND ${Users.state} = ${UserState.ACTIVE}
          )
          SELECT
            date_series.date::text as date,
            COUNT(${Users.id})::int as value,
            (SELECT count FROM current_window) as current
          FROM date_series
          LEFT JOIN ${Users} ON DATE(${Users.createdAt} AT TIME ZONE 'Asia/Seoul') = date_series.date
            AND ${Users.state} = ${UserState.ACTIVE}
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getUsersActive = () =>
        dbr.execute(sql`
          WITH ${dateSeries},
          ${validChanges},
          current_window AS (
            SELECT COUNT(DISTINCT user_id)::int as count
            FROM valid_changes
            WHERE bucket >= ${twentyFourHoursAgo}
              AND bucket < ${now}
          )
          SELECT
            date_series.date::text as date,
            COUNT(DISTINCT vc.user_id)::int as value,
            (SELECT count FROM current_window) as current
          FROM date_series
          LEFT JOIN valid_changes vc ON DATE(vc.bucket AT TIME ZONE 'Asia/Seoul') = date_series.date
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      // Subscription metrics
      // 한 유저의 entitled 행이 겹치는 전환 창(해지 예약·유예 + 시작 경과 예약)에서 이중 합산을 막기 위해
      // 유저당 대표 1건만 합산한다 — 선택 규칙은 selectRepresentativeSubscription 과 동일(ACTIVE 우선, 최신 생성).
      const getSubscriptionsRevenue = () =>
        dbr.execute(sql`
          WITH ${dateSeries},
          ${paidSubscriptions},
          entitled AS (
            SELECT
              date_series.date AS date,
              s.monthly_fee AS monthly_fee,
              row_number() OVER (
                PARTITION BY date_series.date, s.user_id
                ORDER BY (s.state = 'ACTIVE') DESC, s.created_at DESC
              ) AS user_rank
            FROM date_series
            LEFT JOIN paid_subscriptions s ON ${entitledOnDate}
          )
          SELECT
            entitled.date::text as date,
            ROUND(COALESCE(SUM(entitled.monthly_fee), 0))::int as value
          FROM entitled
          WHERE entitled.user_rank = 1
          GROUP BY entitled.date
          ORDER BY entitled.date
        `);

      const getSubscriptionsActive = () =>
        dbr.execute(sql`
          WITH ${dateSeries},
          ${paidSubscriptions}
          SELECT
            date_series.date::text as date,
            COUNT(DISTINCT s.user_id)::int as value
          FROM date_series
          LEFT JOIN paid_subscriptions s ON ${entitledOnDate}
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      // Document metrics
      // 삭제·영구삭제된 문서도 포함한다 — "지금까지 만들어진 문서 수"라는 라이프타임 지표다.
      const getDocumentsTotal = () =>
        dbr.execute(sql`
          WITH ${dateSeries},
          real_documents AS (
            SELECT ${Documents.id} AS id, ${Documents.createdAt} AS created_at
            FROM ${Documents}
            INNER JOIN ${Entities} ON ${Documents.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Entities.createdAt} != ${Sites.createdAt}
          )
          SELECT
            date_series.date::text as date,
            COUNT(rd.id)::int as value
          FROM date_series
          LEFT JOIN real_documents rd ON rd.created_at < ${kstDayEnd}
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      // Character metrics
      // additions 는 세그먼트 단위 순증 코얼레싱 값이라 그로스 타이핑보다 작다. 누적 합은 int4 상한을 넘을 수 있어 bigint 로 낸다.
      const getCharactersInput = () =>
        dbr.execute(sql`
          WITH ${dateSeries},
          ${validChanges}
          SELECT
            date_series.date::text as date,
            COALESCE(SUM(vc.additions), 0)::bigint as value
          FROM date_series
          LEFT JOIN valid_changes vc ON vc.bucket < ${kstDayEnd}
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getCharactersDaily = () =>
        dbr.execute(sql`
          WITH ${dateSeries},
          ${validChanges},
          current_window AS (
            SELECT COALESCE(SUM(additions), 0)::bigint as total
            FROM valid_changes
            WHERE bucket >= ${twentyFourHoursAgo}
              AND bucket < ${now}
          )
          SELECT
            date_series.date::text as date,
            COALESCE(SUM(vc.additions), 0)::bigint as value,
            (SELECT total FROM current_window) as current
          FROM date_series
          LEFT JOIN valid_changes vc ON DATE(vc.bucket AT TIME ZONE 'Asia/Seoul') = date_series.date
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      // System metrics
      const getSystemServiceDays = () =>
        dbr.execute(sql`
          WITH ${dateSeries},
          service_launch AS (
            SELECT MIN(DATE(${Users.createdAt} AT TIME ZONE 'Asia/Seoul')) as launch_date
            FROM ${Users}
            WHERE ${Users.state} = ${UserState.ACTIVE}
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

      // 일별 지표의 current 는 시리즈 마지막 점(진행 중인 달력일)이 아니라 롤링 24시간 값이다.
      const transformToWindowedData = (rows: Record<string, unknown>[]) => {
        const data = rows.map((row) => ({
          date: String(row.date),
          value: Number(row.value),
        }));

        return { data, current: Number(rows.at(-1)?.current ?? 0) };
      };

      const result = {
        // User metrics
        usersTotal: transformToData(usersTotal),
        usersNew: transformToWindowedData(usersNew),
        usersActive: transformToWindowedData(usersActive),

        // Subscription metrics
        subscriptionsRevenue: transformToData(subscriptionsRevenue),
        subscriptionsActive: transformToData(subscriptionsActive),

        // Document metrics
        documentsTotal: transformToData(documentsTotal),

        // Character metrics
        charactersInput: transformToData(charactersInput),
        charactersDaily: transformToWindowedData(charactersDaily),

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
