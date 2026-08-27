import PrismQuestionCard from '../PrismQuestionCard.svelte';
import PrismReviewConfirmCard from '../review/PrismReviewConfirmCard.svelte';
import PrismActionCard from './PrismActionCard.svelte';
import type { ToolRequestMessage } from '@typie/prism';
import type { Component } from 'svelte';
import type { OpenDocumentRegistry } from '$lib/prism/open-documents.svelte';

export type ToolCardProps = {
  message: ToolRequestMessage;
  sessionId: string | null;
  open: boolean;
  disabled: boolean;
  resolve: (input: unknown) => Promise<void>;
};

export type ClientResolverDeps = {
  openDocuments: OpenDocumentRegistry;
};

export const clientResolvers: Record<string, ((deps: ClientResolverDeps) => unknown | Promise<unknown>) | undefined> = {
  'list-open-documents': ({ openDocuments }) => openDocuments.snapshotWhenReady(),
};

export type { ActionBodyProps } from './action-cards.ts';
export { actionCards } from './action-cards.ts';

export const toolCards: Record<string, Component<ToolCardProps> | undefined> = {
  'confirm-review': PrismReviewConfirmCard,
  'ask-user': PrismQuestionCard,
  'delete-entities': PrismActionCard,
  'delete-notes': PrismActionCard,
  'delete-goals': PrismActionCard,
  'update-sharing': PrismActionCard,
  'save-document': PrismActionCard,
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
  'read-sharing': '공개 범위를 확인했어요',
  'read-comments': '코멘트를 읽었어요',
  'list-trash': '휴지통을 살펴봤어요',
  'list-icons': '아이콘 목록을 확인했어요',
  'open-document': '문서를 열었어요',
  'outline-document': '문서 구조를 살펴봤어요',
  'create-folders': '폴더를 만들었어요',
  'create-documents': '문서를 만들었어요',
  'update-documents': '문서 제목을 바꿨어요',
  'update-folders': '폴더 이름을 바꿨어요',
  'edit-document': '문서 파일을 고쳤어요',
  'move-entities': '이동했어요',
  'duplicate-documents': '문서를 복제했어요',
  'create-notes': '노트를 만들었어요',
  'update-notes': '노트를 고쳤어요',
  'attach-notes': '노트를 연결했어요',
  'detach-notes': '노트 연결을 해제했어요',
  'set-goals': '목표를 정했어요',
  'update-icons': '아이콘을 바꿨어요',
  'recover-entities': '휴지통에서 복원했어요',
  'delete-entities': '삭제했어요',
  'delete-notes': '노트를 지웠어요',
  'delete-goals': '목표를 없앴어요',
  'update-sharing': '공개 범위를 바꿨어요',
  'save-document': '문서를 저장했어요',
  read: '파일을 읽었어요',
  grep: '파일을 검색했어요',
  write: '파일을 작성했어요',
  edit: '파일을 고쳤어요',
  list: '파일을 살펴봤어요',
};

export const toolCallFailureLabels: Record<string, string | undefined> = {
  'save-document': '문서를 저장하지 못했어요',
};
