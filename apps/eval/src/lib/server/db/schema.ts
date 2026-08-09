import { integer, primaryKey, sqliteTable, text } from 'drizzle-orm/sqlite-core';
import { TIER_NAMES } from '../../feedback/tiers.ts';

export const FeedbackSessions = sqliteTable('feedback_sessions', {
  id: text('id').primaryKey(),
  refId: text('ref_id').notNull(),
  title: text('title'),
  testerEmail: text('tester_email').notNull(),
  createdAt: integer('created_at', { mode: 'timestamp_ms' }).notNull(),
});

export const ManuscriptVersions = sqliteTable(
  'manuscript_versions',
  {
    sessionId: text('session_id').notNull(),
    version: integer('version').notNull(),
    content: text('content').notNull(),
    charCount: integer('char_count').notNull(),
    importedAt: integer('imported_at', { mode: 'timestamp_ms' }).notNull(),
  },
  (t) => [primaryKey({ columns: [t.sessionId, t.version] })],
);

export const Reviews = sqliteTable(
  'reviews',
  {
    sessionId: text('session_id').notNull(),
    round: integer('round').notNull(),
    prismWorkflowId: text('prism_workflow_id').notNull().unique(),
    status: text('status', { enum: ['running', 'completed', 'failed', 'canceled'] }).notNull(),
    manuscriptVersion: integer('manuscript_version').notNull(),
    startedAt: integer('started_at', { mode: 'timestamp_ms' }).notNull(),
    finishedAt: integer('finished_at', { mode: 'timestamp_ms' }),
    usage: text('usage', { mode: 'json' }),
    result: text('result', { mode: 'json' }),
    error: text('error'),
    events: text('events', { mode: 'json' }),
    tier: text('tier', { enum: TIER_NAMES }).notNull().default('high'),
    modelConfig: text('model_config', { mode: 'json' }),
    questions: text('questions', { mode: 'json' }),
  },
  (t) => [primaryKey({ columns: [t.sessionId, t.round] })],
);

export const Threads = sqliteTable('threads', {
  id: text('id').primaryKey(),
  sessionId: text('session_id').notNull(),
  reviewRound: integer('review_round').notNull(),
  issueIndex: integer('issue_index').notNull(),
  axis: text('axis').notNull(),
  pass: text('pass', { enum: ['critique', 'proofread'] }).notNull(),
  body: text('body'),
  anchors: text('anchors', { mode: 'json' }).notNull(),
  state: text('state', { enum: ['open', 'closed', 'resolved', 'withdrawn'] })
    .notNull()
    .default('open'),
  stateChangedAt: integer('state_changed_at', { mode: 'timestamp_ms' }),
  reaction: text('reaction', { enum: ['up', 'down'] }),
});

export const ThreadComments = sqliteTable('thread_comments', {
  id: text('id').primaryKey(),
  threadId: text('thread_id').notNull(),
  author: text('author', { enum: ['tester', 'ai'] }).notNull(),
  body: text('body').notNull(),
  reviewRound: integer('review_round'),
  createdAt: integer('created_at', { mode: 'timestamp_ms' }).notNull(),
});

export const Reactions = sqliteTable(
  'reactions',
  {
    sessionId: text('session_id').notNull(),
    reviewRound: integer('review_round').notNull(),
    value: text('value', { enum: ['up', 'down'] }).notNull(),
    note: text('note'),
    createdAt: integer('created_at', { mode: 'timestamp_ms' }).notNull(),
  },
  (t) => [primaryKey({ columns: [t.sessionId, t.reviewRound] })],
);
