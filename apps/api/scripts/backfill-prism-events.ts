#!/usr/bin/env node

// prism 대화·워크플로 로그를 타이피 DB 로 적재하는 백필 — 로그마다 인제스트 잡을 1개씩 넣는다.
// 멱등(jobId·유니크)이라 재실행 안전. dry-run 이 기본, --yes 로 enqueue.
//
//   미리보기: SCRIPT=1 NODE_ENV=production doppler run --config prod_local -- node --experimental-strip-types \
//     scripts/backfill-prism-events.ts 2>&1 | tee backfill-prism-events.dry.log
//   실행:     SCRIPT=1 NODE_ENV=production doppler run --config prod_local -- node --experimental-strip-types \
//     scripts/backfill-prism-events.ts --yes 2>&1 | tee backfill-prism-events.log
//
// SCRIPT=1 은 안전장치다 — #/mq/index.ts 에 닿는 임포트가 생기면 그 평가 시점에 워커가 이 프로세스에서
// 돌기 시작한다(mq/bullmq.ts:72, mq/prism-worker.ts:29). NODE_ENV=production 은 필수다 — 레인이 dev 면
// os.hostname(), 아니면 stack 이라(mq/prism-queue.ts:8) 빠뜨리면 아무도 집지 않는 레인에 잡이 쌓인다.
// 출력 첫 줄의 레인 이름이 워커 팟의 레인과 같은지 dry-run 으로 먼저 확인할 것.

process.env.SCRIPT = '1';

const { asc, eq, isNull } = await import('drizzle-orm');
const { db, pg, PrismSessions, PrismWorkflows } = await import('#/db/index.ts');
const { ensureIngest, PRISM_LANE, prismQueue } = await import('#/mq/prism-queue.ts');

const apply = process.argv.includes('--yes');

const sessions = await db
  .select({ id: PrismSessions.id })
  .from(PrismSessions)
  .where(isNull(PrismSessions.deletedAt))
  .orderBy(asc(PrismSessions.id));
const workflows = await db
  .select({ id: PrismWorkflows.id })
  .from(PrismWorkflows)
  .innerJoin(PrismSessions, eq(PrismWorkflows.sessionId, PrismSessions.id))
  .where(isNull(PrismSessions.deletedAt))
  .orderBy(asc(PrismWorkflows.id));

console.log(
  `prism 이벤트 백필 — ${apply ? '실행' : 'DRY RUN'} / 레인 ${PRISM_LANE} / 세션 ${sessions.length}건 / 워크플로 ${workflows.length}건`,
);

if (apply) {
  for (const session of sessions) {
    await ensureIngest({ kind: 'agent', sessionId: session.id });
  }

  for (const workflow of workflows) {
    await ensureIngest({ kind: 'workflow', workflowId: workflow.id });
  }

  console.log(JSON.stringify(await prismQueue.getJobCounts()));
}

// process.exit 은 파이프(tee)로 흘려보낸 출력을 자르므로 exitCode 만 세우고 커넥션을 닫아 자연 종료한다.
// prismQueue 는 ioredis 인스턴스를 주입받은 공유 커넥션이라 close() 가 소켓을 끊지 않는다 — 직접 끊는다.
process.exitCode = 0;
const connection = await prismQueue.getBackend().client;
await prismQueue.close();
connection.disconnect();
await pg.end();
