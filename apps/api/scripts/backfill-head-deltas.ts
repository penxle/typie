#!/usr/bin/env node

// cspell:ignore timestamptz

// 타임라인 head 별 유저 기여 델타 백필. 인접 head 의 character_count 차(net)를 그 head 의 contributor 행의
// additions/deletions 에 기록한다 — 히스토리에는 유저별 귀속 단서가 남아 있지 않아 head 단위 diff 밖에 없다.
//
// 대상은 contributor 가 1명인 head 로 한정한다. 이때는 head diff 가 곧 그 유저의 net 이라 원기록 없이도
// 정확히 복원된다. 다인(2+) head 는 유저별 원기록이 없어 복원 불가라 백필이 손대지 않는다 — 균등 분배하면
// 실제 net 이 0 이던 유저에게 diff 몫이 붙어, 그 유저가 제외를 켜는 순간 있지도 않던 기여가 차감돼 통계가
// 음수로 내려간다.
//
// 손대지 않은 다인 head 의 행이 남는 형태는 둘이다. ① 순수 히스토리 다인 head 는 additions 가 NULL 로 남고
// 기존 경로가 그대로 처리한다 — DocumentHead.excluded 가 null 로 나가 웹 타임라인이 토글을 렌더하지 않고
// updateDocumentHeadExclusion 은 head_delta_unavailable 로 거절한다(contributor 0명 head 와 동형).
// ② 배포 창 fold 가 얹힌 다인 head 는 그 fold 분량의 부분값이 남아 NOT NULL 이고 토글도 살아 있다. 부분값은
// planHeadWrites 가 낸 실제 유저별 net 이라 없는 기여가 아니며 0 이상이고, 전체 기여보다 작아 제외 시
// 과소 차감된다(통계가 덜 깎이는 방향, 음수 불가).
//
// 시간 스코프: additions IS NULL(미기록) 또는 --cutoff(api 배포 시각) 이전 생성 head 의 행. 후자는 배포 창에서
// 신 collect 가 구행에 부분값을 얹었기 때문이다 — changeset.ts 의 contributor upsert 는 충돌 시
// COALESCE(additions, 0) + delta 로 더하므로, NULL 이던 구행이 그 fold 분량만 담은 값으로 바뀌어
// additions IS NULL 스코프에서 빠져나간다. UPDATE 가 SET 덮어쓰기라 이 부분값을 전체 diff 로 되돌린다.
//
// 실행 전제: document_heads.seq 를 확정하는 마이그레이션(0107_document-head-constraints) 이후 —
// 같은 bucket 안의 정렬 타이브레이크가 seq 다.
//
//   미리보기: doppler run --config prod_local -- node scripts/backfill-head-deltas.ts --cutoff <ISO> 2>&1 | tee backfill-head-deltas-dry.log
//   실행:     doppler run --config prod_local -- node scripts/backfill-head-deltas.ts --cutoff <ISO> --yes 2>&1 | tee backfill-head-deltas.log
//
// --cutoff 는 오프셋을 붙인 ISO8601 로 준다(예: 2026-08-07T21:00:00+09:00). 오프셋을 빼면 세션 시간대
// 달력으로 해석돼 경계가 통째로 밀린다.
//
// 재실행 안전성의 근거는 스코프와 SET 이다. ① 값은 매 실행 현재 character_count 에서 다시 계산되고 SET 으로
// 덮어쓰므로 이중 계상이 없다 — 같은 --cutoff 로 몇 번을 돌려도 결과가 같다. ② cutoff 이후 생성된 head 의
// 행은 신 collect 가 이미 정확한 값을 넣었고 additions 가 NOT NULL 이라 스코프에 들어오지 않는다(불가침).
// 체크포인트를 두지 않는 근거도 이것이다 — 중단되면 그냥 다시 돌린다.
//
// ①의 전제는 실행 시각이다. head bucket 은 10분(HEAD_BUCKET_SECONDS)이고 fold 는 같은 bucket 의 최신 head
// 에만 붙으므로, 라이브 bucket 에 걸친 cutoff 이전 head 는 새 유저의 fold 로 contributor 가 1명에서 2명이
// 되어 스코프를 이탈할 수 있다. cutoff + 10분 이후에 실행하면 cutoff 이전 head 의 bucket 이 전부 닫혀
// contributor 수가 고정된다 — 런북의 "배포 후 10분 경과 뒤 실행" 조건이 이것이고, 미리보기와 실제 적용의
// 대상 집합이 같다는 보장도 여기서 나온다.
//
// 출력은 대상 분포 대조가 본체라 반드시 파일로 남긴다(위 tee).

import { parseArgs } from 'node:util';
import { sql } from 'drizzle-orm';
import { db, pg } from '#/db/index.ts';

const { values } = parseArgs({ options: { yes: { type: 'boolean', default: false }, cutoff: { type: 'string' } } });
const dryRun = !values.yes;

const diffCte = sql`
  WITH ordered AS (
    SELECT id, created_at, character_count
         - COALESCE(LAG(character_count) OVER (PARTITION BY document_id ORDER BY bucket, seq), 0) AS diff
    FROM document_heads
  ),
  counts AS (
    SELECT head_id, COUNT(*)::int AS cnt FROM document_head_contributors GROUP BY head_id
  )
`;

const main = async () => {
  const cutoff = values.cutoff;

  if (!cutoff) {
    console.error('--cutoff <ISO8601> (api 배포 시각) 필수 — 이전 생성 head 는 델타를 전체 diff 로 재귀속한다');
    process.exitCode = 1;

    return;
  }

  console.log(dryRun ? 'DRY RUN (실제 적용은 --yes)' : 'APPLY MODE');
  console.log(`cutoff: ${cutoff}`);

  // 대상 술어의 시간 조건은 이 조각 하나를 미리보기와 적용이 공유한다 — 문면이 갈라질 수 없게 만드는 것이 목적이다.
  // 미리보기는 여기에 cnt 필터를 걸지 않고 분포 전체를 뽑아 대상(cnt=1)과 스킵(cnt>=2)을 함께 보여준다.
  const timeScope = sql`(c.additions IS NULL OR o.created_at < ${cutoff}::timestamptz)`;

  const preview = await db.execute<{ cnt: number; rows: number }>(sql`
    ${diffCte}
    SELECT k.cnt, COUNT(*)::int AS rows
    FROM document_head_contributors c
    JOIN ordered o ON o.id = c.head_id
    JOIN counts k ON k.head_id = c.head_id
    WHERE ${timeScope}
    GROUP BY k.cnt ORDER BY k.cnt
  `);

  let targetRows = 0;
  let skippedRows = 0;

  for (const row of preview) {
    const target = row.cnt === 1;

    if (target) {
      targetRows += row.rows;
    } else {
      skippedRows += row.rows;
    }

    console.log(`  contributor ${row.cnt}명 head의 행: ${row.rows} — ${target ? '백필 대상' : '스킵(다인 head — 복원 불가)'}`);
  }

  console.log(`백필 대상 행 합계: ${targetRows} — 적용 시 '갱신 완료' 수와 같아야 한다`);
  console.log(`스킵 행 합계: ${skippedRows} — 다인 head 라 손대지 않는다(순수 히스토리 행은 NULL 유지, 배포 창 부분값은 그대로)`);

  if (!dryRun) {
    const updated = await db.execute(sql`
      ${diffCte}
      UPDATE document_head_contributors c
      SET additions = GREATEST(o.diff, 0) / k.cnt,
          deletions = GREATEST(-o.diff, 0) / k.cnt
      FROM ordered o, counts k
      WHERE o.id = c.head_id AND k.head_id = c.head_id
        AND ${timeScope}
        AND k.cnt = 1
    `);
    console.log(`갱신 완료: ${updated.count}행`);

    const remaining = await db.execute<{ rows: number }>(
      sql`SELECT COUNT(*)::int AS rows FROM document_head_contributors WHERE additions IS NULL`,
    );
    console.log(`잔여 NULL 행(다인 head — 정상): ${remaining[0]?.rows ?? '?'}`);
  }
};

// process.exit 은 파이프(tee)로 흘려보낸 출력을 자르므로 exitCode 만 세우고 커넥션을 닫아 자연 종료한다.
await main();
await pg.end();
