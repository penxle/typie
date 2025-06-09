import dayjs from 'dayjs';
import { sql } from 'drizzle-orm';
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

      const now = dayjs();
      const thirtyDaysAgo = now.subtract(30, 'day');
      const twentyFourHoursAgo = now.subtract(24, 'hour');
      const fortyEightHoursAgo = now.subtract(48, 'hour');

      const getTotalUsers = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
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

      const getNewSignups = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
          ),
          current_period AS (
            SELECT COUNT(${Users.id})::int as count
            FROM ${Users}
            WHERE ${Users.createdAt} >= ${twentyFourHoursAgo.toDate()}
              AND ${Users.createdAt} < ${now.toDate()}
              AND ${Users.state} = ${UserState.ACTIVE}
          ),
          previous_period AS (
            SELECT COUNT(${Users.id})::int as count
            FROM ${Users}
            WHERE ${Users.createdAt} >= ${fortyEightHoursAgo.toDate()}
              AND ${Users.createdAt} < ${twentyFourHoursAgo.toDate()}
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

      const getActiveWriters = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
          ),
          current_period AS (
            SELECT COUNT(DISTINCT ${PostCharacterCountChanges.userId})::int as count
            FROM ${PostCharacterCountChanges}
            INNER JOIN ${Posts} ON ${PostCharacterCountChanges.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${PostCharacterCountChanges.bucket} >= ${twentyFourHoursAgo.toDate()}
              AND ${PostCharacterCountChanges.bucket} < ${now.toDate()}
              AND ${Entities.createdAt} != ${Sites.createdAt}
          ),
          previous_period AS (
            SELECT COUNT(DISTINCT ${PostCharacterCountChanges.userId})::int as count
            FROM ${PostCharacterCountChanges}
            INNER JOIN ${Posts} ON ${PostCharacterCountChanges.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${PostCharacterCountChanges.bucket} >= ${fortyEightHoursAgo.toDate()}
              AND ${PostCharacterCountChanges.bucket} < ${twentyFourHoursAgo.toDate()}
              AND ${Entities.createdAt} != ${Sites.createdAt}
          )
          SELECT 
            date_series.date::text as date,
            CASE 
              WHEN date_series.date = CURRENT_DATE - INTERVAL '1 day' THEN COALESCE((SELECT count FROM previous_period), 0)
              WHEN date_series.date = CURRENT_DATE THEN COALESCE((SELECT count FROM current_period), 0)
              ELSE COALESCE(COUNT(DISTINCT ${PostCharacterCountChanges.userId}), 0)
            END::int as value
          FROM date_series
          LEFT JOIN ${PostCharacterCountChanges} ON DATE(${PostCharacterCountChanges.bucket}) = date_series.date
          LEFT JOIN ${Posts} ON ${PostCharacterCountChanges.postId} = ${Posts.id}
          LEFT JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
          LEFT JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            AND ${Entities.createdAt} != ${Sites.createdAt}
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getMonthlyRecurringRevenue = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
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
              AND ${Subscriptions.expiresAt} >= ${thirtyDaysAgo.toDate()}
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

      const getActiveSubscriptions = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
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

      const getRealPosts = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
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

      const getNewRealPosts = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
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
            WHERE rp.created_at >= ${twentyFourHoursAgo.toDate()}
              AND rp.created_at < ${now.toDate()}
          ),
          previous_period AS (
            SELECT COUNT(rp.id)::int as count
            FROM real_posts rp
            WHERE rp.created_at >= ${fortyEightHoursAgo.toDate()}
              AND rp.created_at < ${twentyFourHoursAgo.toDate()}
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

      const getAveragePostLength = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
          ),
          daily_avg AS (
            SELECT 
              DATE(${Posts.createdAt}) as post_date,
              AVG(${PostContents.characterCount}) as avg_length
            FROM ${Posts}
            INNER JOIN ${PostContents} ON ${Posts.id} = ${PostContents.postId}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Posts.createdAt} >= ${thirtyDaysAgo.toDate()}
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

      const getTotalCharacters = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
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

      const getTotalInputCharacters = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
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

      const getDailyCharacters = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
          ),
          current_period AS (
            SELECT SUM(${PostCharacterCountChanges.additions})::int as total
            FROM ${PostCharacterCountChanges}
            INNER JOIN ${Posts} ON ${PostCharacterCountChanges.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${PostCharacterCountChanges.bucket} >= ${twentyFourHoursAgo.toDate()}
              AND ${PostCharacterCountChanges.bucket} < ${now.toDate()}
              AND ${Entities.createdAt} != ${Sites.createdAt}
          ),
          previous_period AS (
            SELECT SUM(${PostCharacterCountChanges.additions})::int as total
            FROM ${PostCharacterCountChanges}
            INNER JOIN ${Posts} ON ${PostCharacterCountChanges.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${PostCharacterCountChanges.bucket} >= ${fortyEightHoursAgo.toDate()}
              AND ${PostCharacterCountChanges.bucket} < ${twentyFourHoursAgo.toDate()}
              AND ${Entities.createdAt} != ${Sites.createdAt}
          )
          SELECT 
            date_series.date::text as date,
            CASE 
              WHEN date_series.date = CURRENT_DATE - INTERVAL '1 day' THEN COALESCE((SELECT total FROM previous_period), 0)
              WHEN date_series.date = CURRENT_DATE THEN COALESCE((SELECT total FROM current_period), 0)
              ELSE COALESCE(SUM(${PostCharacterCountChanges.additions}), 0)
            END::int as value
          FROM date_series
          LEFT JOIN ${PostCharacterCountChanges} ON DATE(${PostCharacterCountChanges.bucket}) = date_series.date
          LEFT JOIN ${Posts} ON ${PostCharacterCountChanges.postId} = ${Posts.id}
          LEFT JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
          LEFT JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            AND ${Entities.createdAt} != ${Sites.createdAt}
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getTotalReactions = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
          )
          SELECT 
            date_series.date::text as date,
            COALESCE(COUNT(${PostReactions.id}), 0)::int as value
          FROM date_series
          LEFT JOIN ${PostReactions} ON ${PostReactions.createdAt} < (date_series.date + interval '1 day')
          LEFT JOIN ${Posts} ON ${PostReactions.postId} = ${Posts.id}
          LEFT JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
          LEFT JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            AND ${Entities.createdAt} != ${Sites.createdAt}
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getNewReactions = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
          ),
          current_period AS (
            SELECT COUNT(${PostReactions.id})::int as count
            FROM ${PostReactions}
            INNER JOIN ${Posts} ON ${PostReactions.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${PostReactions.createdAt} >= ${twentyFourHoursAgo.toDate()}
              AND ${PostReactions.createdAt} < ${now.toDate()}
              AND ${Entities.createdAt} != ${Sites.createdAt}
          ),
          previous_period AS (
            SELECT COUNT(${PostReactions.id})::int as count
            FROM ${PostReactions}
            INNER JOIN ${Posts} ON ${PostReactions.postId} = ${Posts.id}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${PostReactions.createdAt} >= ${fortyEightHoursAgo.toDate()}
              AND ${PostReactions.createdAt} < ${twentyFourHoursAgo.toDate()}
              AND ${Entities.createdAt} != ${Sites.createdAt}
          )
          SELECT 
            date_series.date::text as date,
            CASE 
              WHEN date_series.date = CURRENT_DATE - INTERVAL '1 day' THEN COALESCE((SELECT count FROM previous_period), 0)
              WHEN date_series.date = CURRENT_DATE THEN COALESCE((SELECT count FROM current_period), 0)
              ELSE COALESCE(COUNT(${PostReactions.id}), 0)
            END::int as value
          FROM date_series
          LEFT JOIN ${PostReactions} ON DATE(${PostReactions.createdAt}) = date_series.date
          LEFT JOIN ${Posts} ON ${PostReactions.postId} = ${Posts.id}
          LEFT JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
          LEFT JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            AND ${Entities.createdAt} != ${Sites.createdAt}
          GROUP BY date_series.date
          ORDER BY date_series.date
        `);

      const getTotalMedia = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
          ),
          media_base AS (
            SELECT 
              ${thirtyDaysAgo.toDate()} as base_date,
              (SELECT COUNT(*) FROM ${Images} WHERE ${Images.createdAt} < ${thirtyDaysAgo.toDate()}) +
              (SELECT COUNT(*) FROM ${Files} WHERE ${Files.createdAt} < ${thirtyDaysAgo.toDate()}) as base_count
          ),
          daily_additions AS (
            SELECT 
              DATE(created_at) as date,
              COUNT(*) as additions
            FROM (
              SELECT ${Images.createdAt} as created_at FROM ${Images} WHERE ${Images.createdAt} >= ${thirtyDaysAgo.toDate()}
              UNION ALL
              SELECT ${Files.createdAt} as created_at FROM ${Files} WHERE ${Files.createdAt} >= ${thirtyDaysAgo.toDate()}
            ) combined
            GROUP BY DATE(created_at)
          )
          SELECT 
            ds.date::text as date,
            (mb.base_count + COALESCE(SUM(da.additions) FILTER (WHERE da.date <= ds.date), 0))::int as value
          FROM date_series ds
          CROSS JOIN media_base mb
          LEFT JOIN daily_additions da ON da.date >= ${thirtyDaysAgo.toDate()}::date AND da.date <= ds.date
          GROUP BY ds.date, mb.base_count
          ORDER BY ds.date
        `);

      const getNewMedia = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
          ),
          current_period AS (
            SELECT 
              (SELECT COUNT(${Images.id}) FROM ${Images} WHERE ${Images.createdAt} >= ${twentyFourHoursAgo.toDate()} AND ${Images.createdAt} < ${now.toDate()}) +
              (SELECT COUNT(${Files.id}) FROM ${Files} WHERE ${Files.createdAt} >= ${twentyFourHoursAgo.toDate()} AND ${Files.createdAt} < ${now.toDate()}) as count
          ),
          previous_period AS (
            SELECT 
              (SELECT COUNT(${Images.id}) FROM ${Images} WHERE ${Images.createdAt} >= ${fortyEightHoursAgo.toDate()} AND ${Images.createdAt} < ${twentyFourHoursAgo.toDate()}) +
              (SELECT COUNT(${Files.id}) FROM ${Files} WHERE ${Files.createdAt} >= ${fortyEightHoursAgo.toDate()} AND ${Files.createdAt} < ${twentyFourHoursAgo.toDate()}) as count
          ),
          daily_media AS (
            SELECT 
              date_series.date,
              COALESCE(COUNT(${Images.id}), 0) + COALESCE(COUNT(${Files.id}), 0) as daily_count
            FROM date_series
            LEFT JOIN ${Images} ON DATE(${Images.createdAt}) = date_series.date
            LEFT JOIN ${Files} ON DATE(${Files.createdAt}) = date_series.date
            GROUP BY date_series.date
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

      const getTotalMediaSize = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
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

      const getUnlistedPrivatePostRatio = () =>
        db.execute(sql`
          WITH date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
          ),
          daily_visibility AS (
            SELECT 
              DATE(${Posts.createdAt}) as post_date,
              COUNT(DISTINCT CASE WHEN ${Entities.visibility} = 'PRIVATE' THEN ${Posts.id} END) as private_count,
              COUNT(DISTINCT ${Posts.id}) as total_count
            FROM ${Posts}
            INNER JOIN ${Entities} ON ${Posts.entityId} = ${Entities.id}
            INNER JOIN ${Sites} ON ${Entities.siteId} = ${Sites.id}
            WHERE ${Posts.createdAt} >= ${thirtyDaysAgo.toDate()}
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

      const getServiceDays = () =>
        db.execute(sql`
          WITH service_launch AS (
            SELECT MIN(${Users.createdAt})::date as launch_date
            FROM ${Users}
            WHERE ${Users.state} = ${UserState.ACTIVE}
          ),
          date_series AS (
            SELECT generate_series(${thirtyDaysAgo.toDate()}, ${now.toDate()}, interval '1 day')::date AS date
          )
          SELECT 
            date_series.date::text as date,
            (date_series.date - sl.launch_date + 1)::int as value
          FROM date_series
          CROSS JOIN service_launch sl
          ORDER BY date_series.date
        `);

      const [
        totalUsersData,
        newSignupsData,
        activeWritersData,
        monthlyRecurringRevenueData,
        activeSubscriptionsData,
        realPostsData,
        newRealPostsData,
        averagePostLengthData,
        totalCharactersData,
        totalInputCharactersData,
        dailyCharactersData,
        totalReactionsData,
        newReactionsData,
        totalMediaData,
        newMediaData,
        totalMediaSizeData,
        unlistedPrivatePostRatioData,
        serviceDaysData,
      ] = await Promise.all([
        getTotalUsers(),
        getNewSignups(),
        getActiveWriters(),
        getMonthlyRecurringRevenue(),
        getActiveSubscriptions(),
        getRealPosts(),
        getNewRealPosts(),
        getAveragePostLength(),
        getTotalCharacters(),
        getTotalInputCharacters(),
        getDailyCharacters(),
        getTotalReactions(),
        getNewReactions(),
        getTotalMedia(),
        getNewMedia(),
        getTotalMediaSize(),
        getUnlistedPrivatePostRatio(),
        getServiceDays(),
      ]);

      const transformToData = (rows: Record<string, unknown>[]) => {
        const data = rows.map((row) => ({
          date: String(row.date),
          value: Number(row.value),
        }));
        return { data, current: data.at(-1)?.value ?? 0 };
      };

      const result = {
        totalUsers: transformToData(totalUsersData.rows),
        newSignups: transformToData(newSignupsData.rows),
        dailyCharacters: transformToData(dailyCharactersData.rows),
        activeWriters: transformToData(activeWritersData.rows),
        monthlyRecurringRevenue: transformToData(monthlyRecurringRevenueData.rows),
        activeSubscriptions: transformToData(activeSubscriptionsData.rows),

        totalCharacters: transformToData(totalCharactersData.rows),
        totalInputCharacters: transformToData(totalInputCharactersData.rows),
        totalPosts: transformToData(realPostsData.rows),
        newPosts: transformToData(newRealPostsData.rows),
        totalReactions: transformToData(totalReactionsData.rows),
        newReactions: transformToData(newReactionsData.rows),
        averagePostLength: transformToData(averagePostLengthData.rows),
        unlistedPrivatePostRatio: transformToData(unlistedPrivatePostRatioData.rows),
        totalMedia: transformToData(totalMediaData.rows),
        newMedia: transformToData(newMediaData.rows),
        totalMediaSize: transformToData(totalMediaSizeData.rows),
        serviceDays: transformToData(serviceDaysData.rows),
      };

      await redis.setex(cacheKey, 3600, JSON.stringify(result));

      return result;
    },
  }),
);
