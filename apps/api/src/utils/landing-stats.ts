import { UserState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { sql } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { dbr, DocumentCharacterCountChanges, Documents, Entities, Sites, Users } from '#/db/index.ts';
import { SYSTEM_USER_ID } from '#/utils/system-actor.ts';
import type { Database, Transaction } from '#/db/index.ts';

const CACHE_KEY = 'stats:landing:v1';
const CACHE_TTL_SECONDS = 3600;
const WINDOW_DAYS = 30;

type Totals = { current: string; previous: string };
type LandingStat = { current: string; perSecond: number };

export type LandingStats = {
  usersTotal: LandingStat;
  documentsTotal: LandingStat;
  charactersInput: LandingStat;
};

const realDocuments = sql`
  real_documents AS (
    SELECT ${Documents.id} AS id, ${Documents.createdAt} AS created_at
    FROM ${Documents}
    INNER JOIN ${Entities} ON ${Documents.entityId} = ${Entities.id}
    INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
    WHERE ${Entities.createdAt} != ${Sites.createdAt}
  )
`;

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

const toStat = ({ current, previous }: Totals): LandingStat => ({
  current,
  perSecond: Math.max(0, (Number(current) - Number(previous)) / WINDOW_DAYS / 86_400),
});

export const computeLandingStats = async (executor: Database | Transaction): Promise<LandingStats> => {
  const today = dayjs().kst();
  const end = sql`((${today.format('YYYY-MM-DD')}::date + 1)::timestamp AT TIME ZONE 'Asia/Seoul')`;
  const start = sql`((${today.subtract(WINDOW_DAYS, 'days').format('YYYY-MM-DD')}::date + 1)::timestamp AT TIME ZONE 'Asia/Seoul')`;

  const [[users], [documents], [characters]] = await Promise.all([
    executor.execute<Totals>(sql`
      SELECT
        COUNT(${Users.id}) FILTER (WHERE ${Users.createdAt} < ${end}) AS current,
        COUNT(${Users.id}) FILTER (WHERE ${Users.createdAt} < ${start}) AS previous
      FROM ${Users}
      WHERE ${Users.state} = ${UserState.ACTIVE}
    `),
    executor.execute<Totals>(sql`
      WITH ${realDocuments}
      SELECT
        COUNT(rd.id) FILTER (WHERE rd.created_at < ${end}) AS current,
        COUNT(rd.id) FILTER (WHERE rd.created_at < ${start}) AS previous
      FROM real_documents rd
    `),
    executor.execute<Totals>(sql`
      WITH ${validChanges}
      SELECT
        COALESCE(SUM(vc.additions) FILTER (WHERE vc.bucket < ${end}), 0)::bigint AS current,
        COALESCE(SUM(vc.additions) FILTER (WHERE vc.bucket < ${start}), 0)::bigint AS previous
      FROM valid_changes vc
    `),
  ]);

  return { usersTotal: toStat(users), documentsTotal: toStat(documents), charactersInput: toStat(characters) };
};

export const refreshLandingStats = async (): Promise<LandingStats> => {
  const stats = await computeLandingStats(dbr);
  await redis.set(CACHE_KEY, JSON.stringify(stats), 'EX', CACHE_TTL_SECONDS);
  return stats;
};

export const getLandingStats = async (): Promise<LandingStats> => {
  const cached = await redis.get(CACHE_KEY);
  if (cached) {
    return JSON.parse(cached) as LandingStats;
  }

  return await refreshLandingStats();
};
