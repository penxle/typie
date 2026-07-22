import { integer, sqliteTable, text, uniqueIndex } from 'drizzle-orm/sqlite-core';
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
  body: text('body').notNull(),
});

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
