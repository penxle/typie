import { queue } from './bullmq.ts';
import { crons } from './tasks/index.ts';
import type { JobsOptions } from 'bullmq';
import type { JobMap, JobName } from './tasks/index.ts';
import type { JobFn } from './types.ts';

// enqueueJob 선언이 아래 top-level await 보다 먼저여야 한다 — 워커는 bullmq.ts 평가 시점에 이미 돌고 있어,
// 등록 대기 중 잡이 처리되면 TDZ 상태의 enqueueJob 을 참조해 죽는다.
export const enqueueJob = async <N extends JobName, F extends JobMap[N]>(
  name: N,
  payload: F extends JobFn<infer P> ? P : never,
  options?: JobsOptions,
) => {
  await queue.add(name, payload, options);
};

// 스케줄러 등록 실패는 부팅 실패로 취급한다(fail-fast) — 조용한 미등록은 전이 크론 부재로 이어진다.
for (const cron of crons) {
  await queue.upsertJobScheduler(cron.name, {
    pattern: cron.pattern,
    tz: 'Asia/Seoul',
  });
}
