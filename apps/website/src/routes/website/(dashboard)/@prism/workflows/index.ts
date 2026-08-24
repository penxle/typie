import { reviewComposerCopy } from '../review/composer.ts';
import PrismReviewPassage from '../review/PrismReviewPassage.svelte';
import type { ToolRequestMessage, Transcript, WorkflowMessage } from '@typie/prism';
import type { Component } from 'svelte';

export type WorkflowBlockProps = {
  message: WorkflowMessage;
  sessionId: string;
  transcript: Transcript;
  requests: ToolRequestMessage[];
  failedIds: ReadonlySet<string>;
  reconnecting: boolean;
  resolve: (agentId: string, toolCallId: string, input: unknown) => Promise<void>;
  onRetry: (toolCallId: string) => void;
};

export type WorkflowComposerCopy = {
  running: string;
  waiting: string;
  stop: string;
  confirmTitle: string;
  confirmMessage: string;
};

export type WorkflowApp = {
  block: Component<WorkflowBlockProps>;
  composer: WorkflowComposerCopy;
};

export const workflowApps: Record<string, WorkflowApp | undefined> = {
  feedback: { block: PrismReviewPassage, composer: reviewComposerCopy },
};
