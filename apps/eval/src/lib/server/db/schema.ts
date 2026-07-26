import { integer, primaryKey, sqliteTable, text, uniqueIndex } from 'drizzle-orm/sqlite-core';
import type {
  RunDocStatus,
  RunKind,
  RunPhase,
  RunStatus,
  StageKey,
  StagePrompt,
  VariantContent,
  VariantStatus,
} from '../../domain/admin-types.ts';
import type { AnalysisPromptContent } from '../../domain/analysis-prompts.ts';
import type { FeedbackLabelMap } from '../../domain/feedback-labels.ts';
import type { JudgmentResult, RoundStage, TaskKind } from '../../domain/types.ts';

const createdAt = () =>
  integer('created_at', { mode: 'timestamp' })
    .notNull()
    .$defaultFn(() => new Date());

export const Documents = sqliteTable('documents', {
  id: text('id').primaryKey(),
  refId: text('ref_id').notNull(),
  content: text('content').notNull(),
  characterCount: integer('character_count').notNull(),
  corpusVersion: text('corpus_version').notNull(),
  genre: text('genre'),
  createdAt: createdAt(),
});

export const Variants = sqliteTable('variants', {
  id: text('id').primaryKey(),
  label: text('label').notNull().unique(),
  round: text('round').notNull(),
  promptVariantId: text('prompt_variant_id'),
  createdAt: createdAt(),
});

export const Runs = sqliteTable('runs', {
  id: text('id').primaryKey(),
  variantId: text('variant_id').notNull(),
  corpusVersion: text('corpus_version').notNull(),
  meta: text('meta', { mode: 'json' }).$type<Record<string, unknown>>(),
  createdAt: createdAt(),
});

export const FeedbackSets = sqliteTable(
  'feedback_sets',
  {
    id: text('id').primaryKey(),
    runId: text('run_id').notNull(),
    documentId: text('document_id').notNull(),
    variantId: text('variant_id').notNull(),
    // 재설계 파이프라인의 작품 총평. 구 파이프라인 세트에는 없다.
    review: text('review', { mode: 'json' }).$type<Record<string, unknown>>(),
  },
  (t) => [uniqueIndex('feedback_sets_run_id_document_id').on(t.runId, t.documentId)],
);

export const Feedbacks = sqliteTable('feedbacks', {
  id: text('id').primaryKey(),
  setId: text('set_id').notNull(),
  ord: integer('ord').notNull(),
  startText: text('start_text').notNull(),
  endText: text('end_text').notNull(),
  matchStart: integer('match_start'),
  matchEnd: integer('match_end'),
  category: text('category'),
  polarity: text('polarity'),
  body: text('body').notNull(),
});

// 피드백 1건이 위치 여러 곳에 대응한다 — 같은 문제가 반복될 때 지적을 묶으면서 발생 위치를
// 모두 보존하기 위해 앵커를 분리했다. feedbacks의 start_text 등 기존 컬럼은 구 파이프라인이
// 아직 쓰고 있어 남겨둔다.
export const FeedbackAnchors = sqliteTable(
  'feedback_anchors',
  {
    id: text('id').primaryKey(),
    feedbackId: text('feedback_id').notNull(),
    ord: integer('ord').notNull(),
    startText: text('start_text').notNull(),
    endText: text('end_text').notNull(),
    matchStart: integer('match_start'),
    matchEnd: integer('match_end'),
    note: text('note'),
  },
  (t) => [uniqueIndex('feedback_anchors_feedback_id_ord').on(t.feedbackId, t.ord)],
);

export const Rounds = sqliteTable('rounds', {
  id: text('id').primaryKey(),
  stage: text('stage').notNull().$type<RoundStage>(),
  config: text('config', { mode: 'json' }).$type<Record<string, unknown>>(),
  createdAt: createdAt(),
});

export const Tasks = sqliteTable('tasks', {
  id: text('id').primaryKey(),
  roundId: text('round_id').notNull(),
  kind: text('kind').notNull().$type<TaskKind>(),
  documentId: text('document_id').notNull(),
  setIds: text('set_ids', { mode: 'json' }).notNull().$type<string[]>(),
  requiredJudgments: integer('required_judgments'),
  golden: integer('golden', { mode: 'boolean' }).notNull().default(false),
  createdAt: createdAt(),
});

export const Judgments = sqliteTable(
  'judgments',
  {
    id: text('id').primaryKey(),
    taskId: text('task_id').notNull(),
    evaluatorEmail: text('evaluator_email').notNull(),
    result: text('result', { mode: 'json' }).$type<JudgmentResult>(),
    falsePositiveFeedbackIds: text('false_positive_feedback_ids', { mode: 'json' }).notNull().$type<string[]>().default([]),
    feedbackLabels: text('feedback_labels', { mode: 'json' }).$type<FeedbackLabelMap>(),
    comment: text('comment'),
    draft: integer('draft', { mode: 'boolean' }).notNull().default(true),
    elapsedSeconds: integer('elapsed_seconds'),
    createdAt: createdAt(),
    updatedAt: integer('updated_at', { mode: 'timestamp' })
      .notNull()
      .$defaultFn(() => new Date()),
  },
  (t) => [uniqueIndex('judgments_task_id_evaluator_email').on(t.taskId, t.evaluatorEmail)],
);

// 재설계 파이프라인 판정 — 세트 단위 5점 척도 대신 피드백 하나하나에 세 가지를 묻는다.
// 라운드 1·2의 5점 척도 데이터는 judgments.result에 그대로 남는다.
// 세 값은 모두 nullable이다: null은 '아직 판정하지 않음'이며, 통과(true)와 구별되어야 한다.
// 기본값을 통과로 두면 평가자가 보지도 않은 피드백이 전부 합격으로 집계된다.
export const FeedbackVerdicts = sqliteTable(
  'feedback_verdicts',
  {
    id: text('id').primaryKey(),
    judgmentId: text('judgment_id').notNull(),
    feedbackId: text('feedback_id').notNull(),
    correct: integer('correct', { mode: 'boolean' }),
    needed: integer('needed', { mode: 'boolean' }),
    useful: integer('useful', { mode: 'boolean' }),
    note: text('note'),
  },
  (t) => [uniqueIndex('feedback_verdicts_judgment_id_feedback_id').on(t.judgmentId, t.feedbackId)],
);

// 작품 총평에 대한 판정. 피드백과 성격이 달라 3항 대신 두 가지만 묻는다.
export const ReviewVerdicts = sqliteTable(
  'review_verdicts',
  {
    id: text('id').primaryKey(),
    judgmentId: text('judgment_id').notNull(),
    setId: text('set_id').notNull(),
    // 이 글을 제대로 읽었는가 (characterization)
    readCorrectly: integer('read_correctly', { mode: 'boolean' }),
    // 어디서부터 손댈지 도움이 되는가 (priority)
    priorityUseful: integer('priority_useful', { mode: 'boolean' }),
    note: text('note'),
  },
  (t) => [uniqueIndex('review_verdicts_judgment_id_set_id').on(t.judgmentId, t.setId)],
);

export const ReleasedTasks = sqliteTable(
  'released_tasks',
  {
    taskId: text('task_id').notNull(),
    evaluatorEmail: text('evaluator_email').notNull(),
    createdAt: createdAt(),
  },
  (t) => [primaryKey({ columns: [t.taskId, t.evaluatorEmail] })],
);

// 재설계 파이프라인(SURVEY→REVIEW→DEDUPE→VERIFY→COMPOSE)의 프롬프트 묶음.
// 기존 prompt_variants는 3단계 구조와 프로덕션 적용 경로에 묶여 있어 별도 테이블로 둔다.
export const AnalysisPromptSets = sqliteTable('analysis_prompt_sets', {
  id: text('id').primaryKey(),
  label: text('label').notNull().unique(),
  note: text('note'),
  content: text('content', { mode: 'json' }).notNull().$type<AnalysisPromptContent>(),
  createdAt: createdAt(),
});

export const Settings = sqliteTable('settings', {
  key: text('key').primaryKey(),
  value: text('value').notNull(),
});

export const PromptVariants = sqliteTable('prompt_variants', {
  id: text('id').primaryKey(),
  label: text('label').notNull().unique(),
  note: text('note'),
  baseVariantId: text('base_variant_id'),
  content: text('content', { mode: 'json' }).notNull().$type<VariantContent>(),
  status: text('status').notNull().$type<VariantStatus>().default('draft'),
  createdAt: createdAt(),
  updatedAt: integer('updated_at', { mode: 'timestamp' })
    .notNull()
    .$defaultFn(() => new Date()),
});

export const PromptApplies = sqliteTable('prompt_applies', {
  id: text('id').primaryKey(),
  promptId: text('prompt_id').notNull(),
  prev: text('prev', { mode: 'json' }).notNull().$type<StagePrompt>(),
  appliedVariantId: text('applied_variant_id').notNull(),
  appliedStage: text('applied_stage').notNull().$type<StageKey>(),
  appliedBy: text('applied_by').notNull(),
  status: text('status').notNull().$type<'applied' | 'failed'>(),
  createdAt: createdAt(),
});

export const PipelineRuns = sqliteTable('pipeline_runs', {
  id: text('id').primaryKey(),
  kind: text('kind').notNull().$type<RunKind>(),
  variantId: text('variant_id'),
  corpusVersion: text('corpus_version').notNull(),
  status: text('status').notNull().$type<RunStatus>(),
  phase: text('phase').$type<RunPhase>(),
  doneChunks: integer('done_chunks').notNull().default(0),
  totalChunks: integer('total_chunks').notNull().default(0),
  doneDocs: integer('done_docs').notNull().default(0),
  totalDocs: integer('total_docs').notNull().default(0),
  promptTokens: integer('prompt_tokens').notNull().default(0),
  completionTokens: integer('completion_tokens').notNull().default(0),
  error: text('error'),
  meta: text('meta', { mode: 'json' }).$type<Record<string, unknown>>(),
  createdAt: createdAt(),
  finishedAt: integer('finished_at', { mode: 'timestamp' }),
});

export const PipelineRunDocs = sqliteTable(
  'pipeline_run_docs',
  {
    id: text('id').primaryKey(),
    runId: text('run_id').notNull(),
    documentId: text('document_id').notNull(),
    workflowInstanceId: text('workflow_instance_id'),
    status: text('status').notNull().$type<RunDocStatus>(),
    doneChunks: integer('done_chunks').notNull().default(0),
    totalChunks: integer('total_chunks').notNull().default(0),
    // 재설계 파이프라인의 진행 단계. 구 파이프라인은 청크 수로 진행률을 내므로 비워 둔다.
    phase: text('phase'),
    error: text('error'),
  },
  (t) => [uniqueIndex('pipeline_run_docs_run_id_document_id').on(t.runId, t.documentId)],
);

export const StageCache = sqliteTable('stage_cache', {
  key: text('key').primaryKey(),
  value: text('value', { mode: 'json' }).notNull(),
  createdAt: createdAt(),
});

export const EvaluatorConsents = sqliteTable('evaluator_consents', {
  email: text('email').primaryKey(),
  createdAt: createdAt(),
});
