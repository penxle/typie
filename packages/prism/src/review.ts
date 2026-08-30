import { z } from 'zod';

export const PRISM_REVIEW_TIERS = ['low', 'medium', 'high'] as const;
export type PrismReviewTierName = (typeof PRISM_REVIEW_TIERS)[number];

export type Anchor = { head: string; tail: string };
export type Pass = 'judgment' | 'stylistic';
export type ReviewVerdict = 'resolved' | 'kept' | 'withdrawn';

export type ReviewIssue = { id?: string; trait: string; pass: Pass; body: string | null; anchors: Anchor[]; thread?: string };

export type ReviewThreadDisposition = {
  threadId: string;
  verdict: ReviewVerdict;
  comment: string | null;
};

export type ReviewPreviousThreadState = 'open' | 'closed' | 'resolved' | 'withdrawn';

export type ReviewPreviousThread = {
  id: string;
  pass: Pass;
  trait: string;
  body: string;
  anchors: { head: string; tail: string }[];
  replies: { body: string; fresh: boolean }[];
  state: ReviewPreviousThreadState;
  issue?: string;
};

export type ReviewPreviousContext = { title: string | null; subtitle: string | null; path: string; threads: ReviewPreviousThread[] };

export type ReviewSeedMapping = { from: string; to: string };

export type ReviewDiagnostics = {
  dropped: { trait: string; count: number }[];
  gaps: (string | null)[];
  scope?: { changed: number; affected: number; out: number };
  withheld: {
    kind: string;
    head: string;
    reading: string | null;
    disposition: 'explained' | 'withheld';
    note: string | null;
  }[];
};

export type ReviewResult = {
  version: 1;
  kind: 'reviewed';
  issues: ReviewIssue[];
  conclusion?: {
    understanding: string | null;
    progress?: string | null;
    strengths?: { body: string | null; anchors: Anchor[] }[];
    patterns: { theme: string | null; body: string; issues: string[] }[];
    priorities: { body: string; issues: string[] }[];
  };
  dispositions?: ReviewThreadDisposition[];
  verdicts?: { trait: string; point: number; note: string | null }[];
  elevations?: { trait: string; body: string; anchors: Anchor[] }[];
  diagnostics?: ReviewDiagnostics;
};

export type ReviewRejectionCategory = 'diary' | 'practical' | 'non-text' | 'non-korean' | 'unprocessable' | 'unclassifiable';

export type ReviewRejection = {
  version: 1;
  kind: 'rejected';
  rejected: { category: ReviewRejectionCategory; message: string; basis: string | null };
};

export type ReviewOutcome = ReviewResult | ReviewRejection;

export const ConfirmHintSchema = z.object({ documentId: z.string().optional(), tier: z.enum(PRISM_REVIEW_TIERS).optional() });
export type ConfirmHint = z.infer<typeof ConfirmHintSchema>;

export const ConfirmDecisionSchema = z.discriminatedUnion('decision', [
  z.object({
    decision: z.literal('confirmed'),
    key: z.string(),
    tier: z.enum(PRISM_REVIEW_TIERS),
    document: z.object({ id: z.string(), title: z.string().nullable(), subtitle: z.string().nullable(), path: z.string() }),
  }),
  z.object({ decision: z.literal('declined') }),
]);

export const ReviewOutcomeEnvelopeSchema = z.discriminatedUnion('kind', [
  z.looseObject({ version: z.literal(1), kind: z.literal('reviewed') }),
  z.looseObject({ version: z.literal(1), kind: z.literal('rejected') }),
]);
