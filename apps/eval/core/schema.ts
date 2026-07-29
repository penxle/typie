import { integer, primaryKey, sqliteTable, text, uniqueIndex } from 'drizzle-orm/sqlite-core';

const createdAt = () =>
  integer('created_at', { mode: 'timestamp' })
    .notNull()
    .$defaultFn(() => new Date());

export type RunStatus = 'pending' | 'running' | 'done' | 'failed' | 'cancelled';

// 문서가 어느 문으로 들어왔는지. 표집은 실서비스의 공개 관문을 지나지만 반입은 refId로 곧장
// 뽑아 비공개 글도 들어온다 — 갈래를 잃으면 지목해 들인 글이 라운드로 흘러가 평가자에게 열린다.
// 기본값이 'intake'인 것은 안전한 쪽으로 넘어지기 위해서다. 표식을 빠뜨린 문서는 코퍼스에
// 섞이는 대신 라운드에서 빠진다.
export type DocumentKind = 'sampled' | 'intake';

export const Documents = sqliteTable('documents', {
  id: text('id').primaryKey(),
  refId: text('ref_id').notNull(),
  content: text('content').notNull(),
  characterCount: integer('character_count').notNull(),
  kind: text('kind').notNull().$type<DocumentKind>().default('intake'),
  genre: text('genre'),
  samplingId: text('sampling_id'),
  createdAt: createdAt(),
});

export const PromptSets = sqliteTable('prompt_sets', {
  id: text('id').primaryKey(),
  generationId: text('generation_id').notNull(),
  label: text('label').notNull().unique(),
  note: text('note'),
  content: text('content', { mode: 'json' }).notNull().$type<Record<string, unknown>>(),
  createdAt: createdAt(),
});

// 실행 = 원고 1편 × 프롬프트 묶음 1개. 워크플로 인스턴스와 1:1이라 배치 층의 카운터 동기화가 없다.
export const Runs = sqliteTable('runs', {
  id: text('id').primaryKey(),
  documentId: text('document_id').notNull(),
  promptSetId: text('prompt_set_id'),
  instanceId: text('instance_id'),
  status: text('status').notNull().$type<RunStatus>(),
  // 진행 중인 위치. 완료·실패 후에는 비운다 — 상태는 status가 말한다.
  phase: text('phase'),
  error: text('error'),
  createdAt: createdAt(),
  finishedAt: integer('finished_at', { mode: 'timestamp' }),
});

// 지적·강점·무혐의·패턴·우선순위·작품 파악이 모두 여기 산다. kind가 종류를, facets가 세대 고유
// 속성을 담는다 — 코어는 facets 안을 들여다보지 않는다.
export const RunItems = sqliteTable('run_items', {
  id: text('id').primaryKey(),
  runId: text('run_id').notNull(),
  kind: text('kind').notNull(),
  // kind 안에서의 순서. 종류마다 정렬의 뜻이 다르다(지적은 본문 순서, 우선순위는 우선도).
  ord: integer('ord').notNull(),
  body: text('body').notNull(),
  facets: text('facets', { mode: 'json' }).notNull().$type<Record<string, string>>(),
});

export const ItemAnchors = sqliteTable(
  'item_anchors',
  {
    id: text('id').primaryKey(),
    itemId: text('item_id').notNull(),
    ord: integer('ord').notNull(),
    startText: text('start_text').notNull(),
    endText: text('end_text').notNull(),
    matchStart: integer('match_start'),
    matchEnd: integer('match_end'),
    note: text('note'),
  },
  (t) => [uniqueIndex('item_anchors_item_id_ord').on(t.itemId, t.ord)],
);

// 패턴·우선순위가 지적을 가리키는 연결. 배열 순번이 아니라 id라, 지적 하나가 빠져도 어긋나지 않는다.
export const ItemLinks = sqliteTable(
  'item_links',
  {
    itemId: text('item_id').notNull(),
    targetItemId: text('target_item_id').notNull(),
    ord: integer('ord').notNull(),
  },
  (t) => [primaryKey({ columns: [t.itemId, t.ord] })],
);

export const PhaseUsage = sqliteTable(
  'phase_usage',
  {
    runId: text('run_id').notNull(),
    phase: text('phase').notNull(),
    calls: integer('calls').notNull().default(0),
    promptTokens: integer('prompt_tokens').notNull().default(0),
    completionTokens: integer('completion_tokens').notNull().default(0),
    // 둘 다 promptTokens에 포함된 값이다(별도 합이 아니다). 캐시 읽기는 입력 단가의 10%,
    // 쓰기는 1.25배라 서로 반대 방향으로 움직인다 — 나눠 세지 않으면 캐싱의 손익을 못 낸다.
    cachedTokens: integer('cached_tokens').notNull().default(0),
    cacheWriteTokens: integer('cache_write_tokens').notNull().default(0),
  },
  (t) => [primaryKey({ columns: [t.runId, t.phase] })],
);

// 비워도 되는 것 — 리플레이 캐시. 재시도는 새 워크플로 인스턴스를 띄우므로 Cloudflare의
// 스텝 캐시가 통하지 않고, 이 표만이 재과금을 막는다.
export const CallCache = sqliteTable(
  'call_cache',
  {
    runId: text('run_id').notNull(),
    key: text('key').notNull(),
    value: text('value', { mode: 'json' }).notNull(),
    createdAt: createdAt(),
  },
  (t) => [primaryKey({ columns: [t.runId, t.key] })],
);

// 비우면 안 되는 것 — 진단 기록. 캐시와 한 표에 섞여 있어 회수 작업 때 함께 날아간 적이 있다.
export const Ledgers = sqliteTable(
  'ledgers',
  {
    runId: text('run_id').notNull(),
    key: text('key').notNull(),
    value: text('value', { mode: 'json' }).notNull(),
    createdAt: createdAt(),
  },
  (t) => [primaryKey({ columns: [t.runId, t.key] })],
);

// active가 라이브 라운드를 명시한다. 최신 created_at을 라이브로 치던 휴리스틱이 진행 중인
// 라운드를 탈취한 사고가 있었다. 여러 라운드를 동시에 열 수 있다.
export const Rounds = sqliteTable('rounds', {
  id: text('id').primaryKey(),
  label: text('label').notNull(),
  evaluationId: text('evaluation_id').notNull(),
  active: integer('active', { mode: 'boolean' }).notNull().default(false),
  createdAt: createdAt(),
});

export const Tasks = sqliteTable(
  'tasks',
  {
    id: text('id').primaryKey(),
    roundId: text('round_id').notNull(),
    runId: text('run_id').notNull(),
    createdAt: createdAt(),
  },
  (t) => [uniqueIndex('tasks_round_id_run_id').on(t.roundId, t.runId)],
);

// task_id가 unique다 — 중복 배정이 없으므로 한 일감에 판정 하나이고, 이 제약이 동시 배정의
// 경합을 막는다. payload는 결과 전체에 대한 답(도움도·총평 축·코멘트)이다.
export const Judgments = sqliteTable('judgments', {
  id: text('id').primaryKey(),
  taskId: text('task_id').notNull().unique(),
  evaluatorEmail: text('evaluator_email').notNull(),
  draft: integer('draft', { mode: 'boolean' }).notNull().default(true),
  // 확정된 평가 단계 수. 마지막 단계 제출에서 draft가 내려간다 — 앞 단계는 확정됐고 뒤 단계가
  // 진행 중인 상태를 draft 하나로는 담을 수 없어 명시 컬럼으로 둔다.
  stage: integer('stage').notNull().default(0),
  payload: text('payload', { mode: 'json' }).notNull().$type<Record<string, unknown>>(),
  elapsedSeconds: integer('elapsed_seconds').notNull().default(0),
  createdAt: createdAt(),
  updatedAt: integer('updated_at', { mode: 'timestamp' })
    .notNull()
    .$defaultFn(() => new Date()),
});

export const JudgmentItems = sqliteTable(
  'judgment_items',
  {
    id: text('id').primaryKey(),
    judgmentId: text('judgment_id').notNull(),
    itemId: text('item_id').notNull(),
    payload: text('payload', { mode: 'json' }).notNull().$type<Record<string, unknown>>(),
  },
  (t) => [uniqueIndex('judgment_items_judgment_id_item_id').on(t.judgmentId, t.itemId)],
);

export const TaskReleases = sqliteTable(
  'task_releases',
  {
    taskId: text('task_id').notNull(),
    evaluatorEmail: text('evaluator_email').notNull(),
    createdAt: createdAt(),
  },
  (t) => [primaryKey({ columns: [t.taskId, t.evaluatorEmail] })],
);

// 동의만으로는 평가자가 되지 않는다 — 어드민이 evaluating을 켜야 배정이 열린다.
export const Evaluators = sqliteTable('evaluators', {
  email: text('email').primaryKey(),
  evaluating: integer('evaluating', { mode: 'boolean' }).notNull().default(false),
  consentedAt: integer('consented_at', { mode: 'timestamp' })
    .notNull()
    .$defaultFn(() => new Date()),
});

export const Samplings = sqliteTable('samplings', {
  id: text('id').primaryKey(),
  instanceId: text('instance_id'),
  status: text('status').notNull().$type<RunStatus>(),
  phase: text('phase'),
  size: integer('size').notNull(),
  error: text('error'),
  createdAt: createdAt(),
  finishedAt: integer('finished_at', { mode: 'timestamp' }),
});

export const Settings = sqliteTable('settings', {
  key: text('key').primaryKey(),
  value: text('value').notNull(),
});
