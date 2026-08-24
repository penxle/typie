import PrismQuestionCard from '../PrismQuestionCard.svelte';
import PrismReviewConfirmCard from '../review/PrismReviewConfirmCard.svelte';
import PrismActionCard from './PrismActionCard.svelte';
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

export type { ActionBodyProps } from './action-cards.ts';
export { actionCards } from './action-cards.ts';

export const toolCards: Record<string, Component<ToolCardProps> | undefined> = {
  'confirm-review': PrismReviewConfirmCard,
  'ask-user': PrismQuestionCard,
  'delete-entities': PrismActionCard,
  'delete-note': PrismActionCard,
  'delete-goal': PrismActionCard,
  'update-sharing': PrismActionCard,
};

export const toolCallLabels: Record<string, string | undefined> = {
  'list-open-documents': '열린 문서를 확인했어요',
  'search-entities': '스페이스를 검색했어요',
  'list-entities': '스페이스를 살펴봤어요',
  'read-document': '문서를 읽었어요',
  'list-notes': '노트를 살펴봤어요',
  'read-note': '노트를 읽었어요',
  'read-stats': '글쓰기 기록을 살펴봤어요',
  'read-goals': '목표를 살펴봤어요',
  'read-sharing': '공개 상태를 확인했어요',
  'read-comments': '댓글을 읽었어요',
  'list-trash': '휴지통을 살펴봤어요',
  'list-icons': '아이콘 목록을 확인했어요',
  'create-folder': '폴더를 만들었어요',
  'create-document': '문서를 만들었어요',
  'rename-folder': '폴더 이름을 바꿨어요',
  'move-entities': '위치를 옮겼어요',
  'duplicate-document': '문서를 복제했어요',
  'create-note': '노트를 만들었어요',
  'update-note': '노트를 고쳤어요',
  'attach-note': '노트를 연결했어요',
  'detach-note': '노트 연결을 풀었어요',
  'set-goal': '목표를 정했어요',
  'update-icon': '아이콘을 바꿨어요',
  'recover-entity': '휴지통에서 되살렸어요',
  'delete-entities': '휴지통으로 보냈어요',
  'delete-note': '노트를 지웠어요',
  'delete-goal': '목표를 없앴어요',
  'update-sharing': '공개 범위를 바꿨어요',
};
