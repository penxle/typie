import { GetCostAndUsageCommand } from '@aws-sdk/client-cost-explorer';
import dayjs from 'dayjs';
import { sql } from 'drizzle-orm';
import ky from 'ky';
import { redis } from '@/cache';
import {
  db,
  Entities,
  Files,
  Images,
  Plans,
  PostCharacterCountChanges,
  PostContents,
  PostReactions,
  Posts,
  Sites,
  Subscriptions,
  Users,
} from '@/db';
import { SubscriptionState, UserState } from '@/enums';
import { env } from '@/env';
import * as aws from '@/external/aws';
import { builder } from '../builder';

builder.queryField('stats', (t) =>
  t.field({
    type: 'JSON',
    resolve: async () => {
      const cacheKey = 'stats';

      const cached = await redis.get(cacheKey);
      if (cached) {
        return JSON.parse(cached);
      }

      const current = dayjs();
      const now = current.toISOString();
      const thirtyDaysAgo = current.subtract(30, 'days').toISOString();
      const twentyFourHoursAgo = current.subtract(24, 'hours').toISOString();
      const fortyEightHoursAgo = current.subtract(48, 'hours').toISOString();

      const getGitStatistics = async () => {
        const oneWeekAgo = current.subtract(7, 'days');

        const query = `
          query($owner: String!, $repo: String!, $since: GitTimestamp!) {
            repository(owner: $owner, name: $repo) {
              defaultBranchRef {
                target {
                  ... on Commit {
                    totalCommits: history {
                      totalCount
                    }
                    weeklyCommits: history(since: $since) {
                      totalCount
                    }
                  }
                }
              }
            }
          }
        `;

        const variables = {
          owner: 'penxle',
          repo: 'typie',
          since: oneWeekAgo,
        };

        try {
          const { data } = await ky
            .post('https://api.github.com/graphql', {
              headers: {
                Authorization: `Bearer ${env.GITHUB_TOKEN}`,
              },
              json: { query, variables },
            })
            .json<{
              data: {
                repository?: {
                  defaultBranchRef?: { target?: { totalCommits: { totalCount: number }; weeklyCommits: { totalCount: number } } };
                };
              };
            }>();

          if (!data?.repository?.defaultBranchRef?.target) {
            throw new Error('GitHub API response invalid');
          }

          return {
            totalCommits: data.repository.defaultBranchRef.target.totalCommits.totalCount,
            weeklyCommits: data.repository.defaultBranchRef.target.weeklyCommits.totalCount,
          };
        } catch {
          return {
            totalCommits: 0,
            weeklyCommits: 0,
          };
        }
      };

      const getUsdToKrwRate = async (): Promise<number> => {
        const cacheKey = 'usd-krw-rate';

        const cached = await redis.get(cacheKey);
        if (cached) {
          return Number.parseFloat(cached);
        }

        try {
          const data = await ky.get('https://open.er-api.com/v6/latest/USD').json<{ rates: { KRW: number } }>();
          const rate = data.rates.KRW;

          if (rate && typeof rate === 'number') {
            await redis.setex(cacheKey, 86_400, rate.toString());
            return rate;
          }

          throw new Error('Invalid rate response');
        } catch {
          return 1350;
        }
      };

      const getInfraCost = async () => {
        try {
          const command = new GetCostAndUsageCommand({
            TimePeriod: {
              Start: current.subtract(30, 'days').format('YYYY-MM-DD'),
              End: current.format('YYYY-MM-DD'),
            },
            Granularity: 'MONTHLY',
            Metrics: ['BlendedCost'],
          });

          const response = await aws.costExplorer.send(command);

          if (!response.ResultsByTime?.[0]?.Total?.BlendedCost?.Amount) {
            throw new Error('Cost Explorer response invalid');
          }

          const usdAmount = Number.parseFloat(response.ResultsByTime[0].Total.BlendedCost.Amount);
          const usdToKrw = await getUsdToKrwRate();

          return Math.round(usdAmount * usdToKrw);
        } catch {
          return 0;
        }
      };

      // User metrics
      const getUsersTotal = () =>
        db.execute(sql`
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
        db.execute(sql`
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
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          current_period AS (
            SELECT COUNT(DISTINCT ${PostCharacterCountChanges.userId})::int as count
            FROM ${PostCharacterCountChanges}
            INNER JOIN ${Posts} ON ${PostCharacterCountChanges.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${PostCharacterCountChanges.bucket} >= ${twentyFourHoursAgo}
              AND ${PostCharacterCountChanges.bucket} < ${now}
              AND ${Entities.createdAt} != ${Sites.createdAt}
          ),
          previous_period AS (
            SELECT COUNT(DISTINCT ${PostCharacterCountChanges.userId})::int as count
            FROM ${PostCharacterCountChanges}
            INNER JOIN ${Posts} ON ${PostCharacterCountChanges.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${PostCharacterCountChanges.bucket} >= ${fortyEightHoursAgo}
              AND ${PostCharacterCountChanges.bucket} < ${twentyFourHoursAgo}
              AND ${Entities.createdAt} != ${Sites.createdAt}
          ),
          valid_user_activities AS (
            SELECT DISTINCT ${PostCharacterCountChanges.userId}, ${PostCharacterCountChanges.bucket}
            FROM ${PostCharacterCountChanges}
            INNER JOIN ${Posts} ON ${PostCharacterCountChanges.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Entities.createdAt} != ${Sites.createdAt}
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
      const getSubscriptionsRevenue = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          active_subscriptions AS (
            SELECT 
              ${Subscriptions.id},
              ${Subscriptions.startsAt} AS starts_at,
              ${Subscriptions.expiresAt} AS expires_at,
              CASE 
                WHEN ${Plans.interval} = 'MONTHLY' THEN ${Plans.fee}
                WHEN ${Plans.interval} = 'YEARLY' THEN ${Plans.fee} / 12
                ELSE 0
              END AS monthly_fee
            FROM ${Subscriptions}
            INNER JOIN ${Plans} ON ${Subscriptions.planId} = ${Plans.id}
            WHERE ${Subscriptions.state} IN (${SubscriptionState.ACTIVE}, ${SubscriptionState.WILL_EXPIRE}, ${SubscriptionState.IN_GRACE_PERIOD})
              AND ${Subscriptions.expiresAt} >= ${thirtyDaysAgo}
          )
          SELECT 
            date_series.date::text as date,
            COALESCE(SUM(active_sub.monthly_fee), 0)::int as value
          FROM date_series
          LEFT JOIN active_subscriptions active_sub ON active_sub.starts_at <= date_series.date
            AND active_sub.expires_at >= date_series.date
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getSubscriptionsActive = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          )
          SELECT 
            date_series.date::text as date,
            COALESCE(COUNT(${Subscriptions.id}), 0)::int as value
          FROM date_series
          LEFT JOIN ${Subscriptions} ON ${Subscriptions.startsAt} <= date_series.date
            AND ${Subscriptions.expiresAt} >= date_series.date
            AND ${Subscriptions.state} IN (${SubscriptionState.ACTIVE}, ${SubscriptionState.WILL_EXPIRE}, ${SubscriptionState.IN_GRACE_PERIOD})
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      // Post metrics
      const getPostsTotal = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          real_posts AS (
            SELECT DISTINCT ${Posts.id}, ${Posts.createdAt} AS created_at
            FROM ${Posts}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Entities.createdAt} != ${Sites.createdAt}
          )
          SELECT 
            date_series.date::text as date,
            COALESCE(COUNT(rp.id), 0)::int as value
          FROM date_series
          LEFT JOIN real_posts rp ON rp.created_at < (date_series.date + interval '1 day')
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getPostsNew = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          real_posts AS (
            SELECT DISTINCT ${Posts.id}, ${Posts.createdAt} AS created_at
            FROM ${Posts}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Entities.createdAt} != ${Sites.createdAt}
          ),
          current_period AS (
            SELECT COUNT(rp.id)::int as count
            FROM real_posts rp
            WHERE rp.created_at >= ${twentyFourHoursAgo}
              AND rp.created_at < ${now}
          ),
          previous_period AS (
            SELECT COUNT(rp.id)::int as count
            FROM real_posts rp
            WHERE rp.created_at >= ${fortyEightHoursAgo}
              AND rp.created_at < ${twentyFourHoursAgo}
          )
          SELECT 
            date_series.date::text as date,
            CASE 
              WHEN date_series.date = CURRENT_DATE - INTERVAL '1 day' THEN COALESCE((SELECT count FROM previous_period), 0)
              WHEN date_series.date = CURRENT_DATE THEN COALESCE((SELECT count FROM current_period), 0)
              ELSE COALESCE(COUNT(rp.id), 0)
            END::int as value
          FROM date_series
          LEFT JOIN real_posts rp ON DATE(rp.created_at) = date_series.date
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getPostsAverageLength = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          daily_avg AS (
            SELECT 
              DATE(${Posts.createdAt}) as post_date,
              AVG(${PostContents.characterCount}) as avg_length
            FROM ${Posts}
            INNER JOIN ${PostContents} ON ${Posts.id} = ${PostContents.postId}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Posts.createdAt} >= ${thirtyDaysAgo}
              AND ${Entities.createdAt} != ${Sites.createdAt}
            GROUP BY DATE(${Posts.createdAt})
          )
          SELECT 
            date_series.date::text as date,
            COALESCE(ROUND(da.avg_length), 0)::int as value
          FROM date_series
          LEFT JOIN daily_avg da ON da.post_date = date_series.date
          GROUP BY date_series.date, da.avg_length
          ORDER BY date_series.date
        `);

      const getPostsPrivateRatio = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          daily_visibility AS (
            SELECT 
              DATE(${Posts.createdAt}) as post_date,
              COUNT(DISTINCT CASE WHEN ${Entities.visibility} = 'PRIVATE' THEN ${Posts.id} END) as private_count,
              COUNT(DISTINCT ${Posts.id}) as total_count
            FROM ${Posts}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Posts.createdAt} >= ${thirtyDaysAgo}
              AND ${Entities.createdAt} != ${Sites.createdAt}
            GROUP BY DATE(${Posts.createdAt})
          )
          SELECT 
            date_series.date::text as date,
            COALESCE(
              CASE 
                WHEN dv.total_count > 0 THEN ROUND((dv.private_count::float / dv.total_count::float) * 100)
                ELSE 0 
              END, 
              0
            )::int as value
          FROM date_series
          LEFT JOIN daily_visibility dv ON dv.post_date = date_series.date
          GROUP BY date_series.date, dv.private_count, dv.total_count
          ORDER BY date_series.date
        `);

      // Character metrics
      const getCharactersTotal = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          )
          SELECT 
            date_series.date::text as date,
            COALESCE(SUM(${PostContents.characterCount}), 0)::int as value
          FROM date_series
          LEFT JOIN ${Posts} ON ${Posts.createdAt} < (date_series.date + interval '1 day')
          LEFT JOIN ${PostContents} ON ${Posts.id} = ${PostContents.postId}
          LEFT JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
          LEFT JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
          WHERE ${Entities.createdAt} != ${Sites.createdAt} OR ${Entities.createdAt} IS NULL
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getCharactersInput = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          )
          SELECT 
            date_series.date::text as date,
            COALESCE(SUM(${PostCharacterCountChanges.additions}), 0)::int as value
          FROM date_series
          LEFT JOIN ${PostCharacterCountChanges} ON ${PostCharacterCountChanges.bucket} < (date_series.date + interval '1 day')
          LEFT JOIN ${Posts} ON ${PostCharacterCountChanges.postId} = ${Posts.id}
          LEFT JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
          LEFT JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
          WHERE ${Entities.createdAt} != ${Sites.createdAt} OR ${Entities.createdAt} IS NULL
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getCharactersDaily = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          current_period AS (
            SELECT SUM(${PostCharacterCountChanges.additions})::int as total
            FROM ${PostCharacterCountChanges}
            INNER JOIN ${Posts} ON ${PostCharacterCountChanges.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${PostCharacterCountChanges.bucket} >= ${twentyFourHoursAgo}
              AND ${PostCharacterCountChanges.bucket} < ${now}
              AND ${Entities.createdAt} != ${Sites.createdAt}
          ),
          previous_period AS (
            SELECT SUM(${PostCharacterCountChanges.additions})::int as total
            FROM ${PostCharacterCountChanges}
            INNER JOIN ${Posts} ON ${PostCharacterCountChanges.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${PostCharacterCountChanges.bucket} >= ${fortyEightHoursAgo}
              AND ${PostCharacterCountChanges.bucket} < ${twentyFourHoursAgo}
              AND ${Entities.createdAt} != ${Sites.createdAt}
          ),
          valid_character_changes AS (
            SELECT ${PostCharacterCountChanges.bucket}, ${PostCharacterCountChanges.additions}
            FROM ${PostCharacterCountChanges}
            INNER JOIN ${Posts} ON ${PostCharacterCountChanges.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Entities.createdAt} != ${Sites.createdAt}
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

      // Reaction metrics
      const getReactionsTotal = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          valid_reactions AS (
            SELECT ${PostReactions.id}, ${PostReactions.createdAt}
            FROM ${PostReactions}
            INNER JOIN ${Posts} ON ${PostReactions.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Entities.createdAt} != ${Sites.createdAt}
          )
          SELECT 
            date_series.date::text as date,
            COALESCE(COUNT(vr.id), 0)::int as value
          FROM date_series
          LEFT JOIN valid_reactions vr ON vr.created_at < (date_series.date + interval '1 day')
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getReactionsNew = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          current_period AS (
            SELECT COUNT(${PostReactions.id})::int as count
            FROM ${PostReactions}
            INNER JOIN ${Posts} ON ${PostReactions.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${PostReactions.createdAt} >= ${twentyFourHoursAgo}
              AND ${PostReactions.createdAt} < ${now}
              AND ${Entities.createdAt} != ${Sites.createdAt}
          ),
          previous_period AS (
            SELECT COUNT(${PostReactions.id})::int as count
            FROM ${PostReactions}
            INNER JOIN ${Posts} ON ${PostReactions.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${PostReactions.createdAt} >= ${fortyEightHoursAgo}
              AND ${PostReactions.createdAt} < ${twentyFourHoursAgo}
              AND ${Entities.createdAt} != ${Sites.createdAt}
          ),
          valid_reactions AS (
            SELECT ${PostReactions.id}, ${PostReactions.createdAt}
            FROM ${PostReactions}
            INNER JOIN ${Posts} ON ${PostReactions.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Entities.createdAt} != ${Sites.createdAt}
          )
          SELECT 
            date_series.date::text as date,
            CASE 
              WHEN date_series.date = CURRENT_DATE - INTERVAL '1 day' THEN COALESCE((SELECT count FROM previous_period), 0)
              WHEN date_series.date = CURRENT_DATE THEN COALESCE((SELECT count FROM current_period), 0)
              ELSE COALESCE(COUNT(vr.id), 0)
            END::int as value
          FROM date_series
          LEFT JOIN valid_reactions vr ON DATE(vr.created_at) = date_series.date
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      // Media metrics
      const getMediaTotal = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          media_base AS (
            SELECT 
              ${thirtyDaysAgo} as base_date,
              (SELECT COUNT(*) FROM ${Images} WHERE ${Images.createdAt} < ${thirtyDaysAgo}) +
              (SELECT COUNT(*) FROM ${Files} WHERE ${Files.createdAt} < ${thirtyDaysAgo}) as base_count
          ),
          daily_additions AS (
            SELECT 
              DATE(created_at) as date,
              COUNT(*) as additions
            FROM (
              SELECT ${Images.createdAt} as created_at FROM ${Images} WHERE ${Images.createdAt} >= ${thirtyDaysAgo}
              UNION ALL
              SELECT ${Files.createdAt} as created_at FROM ${Files} WHERE ${Files.createdAt} >= ${thirtyDaysAgo}
            ) combined
            GROUP BY DATE(created_at)
          )
          SELECT 
            ds.date::text as date,
            (mb.base_count + COALESCE(SUM(da.additions) FILTER (WHERE da.date <= ds.date), 0))::int as value
          FROM date_series ds
          CROSS JOIN media_base mb
          LEFT JOIN daily_additions da ON da.date >= ${thirtyDaysAgo}::date AND da.date <= ds.date
          GROUP BY ds.date, mb.base_count
          ORDER BY ds.date
        `);

      const getMediaNew = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          ),
          current_period AS (
            SELECT 
              (SELECT COUNT(${Images.id}) FROM ${Images} WHERE ${Images.createdAt} >= ${twentyFourHoursAgo} AND ${Images.createdAt} < ${now}) +
              (SELECT COUNT(${Files.id}) FROM ${Files} WHERE ${Files.createdAt} >= ${twentyFourHoursAgo} AND ${Files.createdAt} < ${now}) as count
          ),
          previous_period AS (
            SELECT 
              (SELECT COUNT(${Images.id}) FROM ${Images} WHERE ${Images.createdAt} >= ${fortyEightHoursAgo} AND ${Images.createdAt} < ${twentyFourHoursAgo}) +
              (SELECT COUNT(${Files.id}) FROM ${Files} WHERE ${Files.createdAt} >= ${fortyEightHoursAgo} AND ${Files.createdAt} < ${twentyFourHoursAgo}) as count
          ),
          daily_media AS (
            SELECT 
              date_series.date,
              (SELECT COUNT(*) FROM ${Images} WHERE DATE(${Images.createdAt}) = date_series.date) +
              (SELECT COUNT(*) FROM ${Files} WHERE DATE(${Files.createdAt}) = date_series.date) as daily_count
            FROM date_series
          )
          SELECT 
            date_series.date::text as date,
            CASE 
              WHEN date_series.date = CURRENT_DATE - INTERVAL '1 day' THEN COALESCE((SELECT count FROM previous_period), 0)
              WHEN date_series.date = CURRENT_DATE THEN COALESCE((SELECT count FROM current_period), 0)
              ELSE COALESCE(dm.daily_count, 0)
            END::int as value
          FROM date_series
          LEFT JOIN daily_media dm ON dm.date = date_series.date
          ORDER BY date_series.date
        `);

      const getMediaTotalSize = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo}, ${now}, interval '1 day')::date AS date
          )
          SELECT 
            date_series.date::text as date,
            COALESCE(
              (SELECT SUM(${Images.size}) FROM ${Images} WHERE ${Images.createdAt} < (date_series.date + interval '1 day')) +
              (SELECT SUM(${Files.size}) FROM ${Files} WHERE ${Files.createdAt} < (date_series.date + interval '1 day')),
              0
            )::bigint as value
          FROM date_series
          ORDER BY date_series.date
        `);

      // System metrics
      const getSystemServiceDays = () =>
        db.execute(sql`
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
        postsTotal,
        postsNew,
        postsAverageLength,
        postsPrivateRatio,
        charactersTotal,
        charactersInput,
        charactersDaily,
        reactionsTotal,
        reactionsNew,
        mediaTotal,
        mediaNew,
        mediaTotalSize,
        systemServiceDays,
        gitStatistics,
        infraCost,
      ] = await Promise.all([
        getUsersTotal(),
        getUsersNew(),
        getUsersActive(),
        getSubscriptionsRevenue(),
        getSubscriptionsActive(),
        getPostsTotal(),
        getPostsNew(),
        getPostsAverageLength(),
        getPostsPrivateRatio(),
        getCharactersTotal(),
        getCharactersInput(),
        getCharactersDaily(),
        getReactionsTotal(),
        getReactionsNew(),
        getMediaTotal(),
        getMediaNew(),
        getMediaTotalSize(),
        getSystemServiceDays(),
        getGitStatistics(),
        getInfraCost(),
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

        // Post metrics
        postsTotal: transformToData(postsTotal),
        postsNew: transformToData(postsNew),
        postsAverageLength: transformToData(postsAverageLength),
        postsPrivateRatio: transformToData(postsPrivateRatio),

        // Character metrics
        charactersTotal: transformToData(charactersTotal),
        charactersInput: transformToData(charactersInput),
        charactersDaily: transformToData(charactersDaily),

        // Reaction metrics
        reactionsTotal: transformToData(reactionsTotal),
        reactionsNew: transformToData(reactionsNew),

        // Media metrics
        mediaTotal: transformToData(mediaTotal),
        mediaNew: transformToData(mediaNew),
        mediaTotalSize: transformToData(mediaTotalSize),

        // System metrics
        systemServiceDays: transformToData(systemServiceDays),

        // External metrics
        gitTotalCommits: gitStatistics.totalCommits,
        gitWeeklyCommits: gitStatistics.weeklyCommits,
        infraMonthlyCost: infraCost,
      };

      await redis.setex(cacheKey, 3600, JSON.stringify(result));

      return result;
    },
  }),
);
