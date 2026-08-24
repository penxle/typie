import PrismQuestionCard from '../PrismQuestionCard.svelte';
import PrismReviewConfirmCard from '../review/PrismReviewConfirmCard.svelte';
import type { ToolRequestMessage } from '@typie/prism';
import type { Component } from 'svelte';
import type { OpenDocumentRegistry } from '$lib/prism/open-documents.svelte';

export type ToolCardProps = {
  message: ToolRequestMessage;
  open: boolean;
  resolve: (input: unknown) => Promise<void>;
};

export type ClientResolverDeps = {
  openDocuments: OpenDocumentRegistry;
};

export const clientResolvers: Record<string, ((deps: ClientResolverDeps) => unknown) | undefined> = {
  'list-open-documents': ({ openDocuments }) => openDocuments.snapshot(),
};

export const toolCards: Record<string, Component<ToolCardProps> | undefined> = {
  'confirm-review': PrismReviewConfirmCard,
  'ask-user': PrismQuestionCard,
};

export const toolCallLabels: Record<string, string | undefined> = {
  'list-open-documents': '열린 원고를 확인했어요',
};
