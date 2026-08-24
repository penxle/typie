import PrismDocumentCard from './PrismDocumentCard.svelte';
import type { Component } from 'svelte';

export type MarkdownCardProps = {
  text: string;
  pending: boolean;
  settled: boolean;
};

export const markdownCards: Record<string, Component<MarkdownCardProps> | undefined> = {
  document: PrismDocumentCard,
};
