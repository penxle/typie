import { sql } from 'drizzle-orm';
import nc from 'node-cron';
import { rapidhash } from 'rapidhash-js';
import { db } from '@/db';
import { enqueueJob } from './publisher';
import { crons } from './tasks';

for (const cron of crons) {
  nc.schedule(
    cron.pattern,
    async () => {
      await db.transaction(async (tx) => {
        const hash = BigInt(rapidhash(cron.name)) % BigInt('9223372036854775807');
        const [{ locked }] = await tx.execute(sql`SELECT pg_try_advisory_xact_lock(${hash}) as locked`);

        if (!locked) {
          return;
        }

        await enqueueJob(cron.name as never, null as never);
      });
    },
    { name: cron.name, timezone: 'Asia/Seoul' },
  );
}
