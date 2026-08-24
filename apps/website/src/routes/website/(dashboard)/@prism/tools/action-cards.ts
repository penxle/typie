import { ToolFailureSchema } from '@typie/prism';
import { css } from '@typie/styled-system/css';
import GlobeIcon from '~icons/lucide/globe';
import TargetIcon from '~icons/lucide/target';
import TrashIcon from '~icons/lucide/trash-2';
import DeleteEntitiesBody from './DeleteEntitiesBody.svelte';
import DeleteGoalBody from './DeleteGoalBody.svelte';
import DeleteNoteBody from './DeleteNoteBody.svelte';
import UpdateSharingBody from './UpdateSharingBody.svelte';
import type { ToolRequestMessage } from '@typie/prism';
import type { Component } from 'svelte';

export type ActionBodyProps = { input: unknown; result: unknown; onReady: (ready: boolean) => void };

export type ActionCard = {
  title: string;
  icon: Component;
  body: Component<ActionBodyProps>;
  confirmLabel: string;
  action: 'danger' | 'primary';
  doneLabel: string;
};

export const actionCards: Record<string, ActionCard | undefined> = {
  'delete-entities': {
    title: '삭제할까요?',
    icon: TrashIcon,
    body: DeleteEntitiesBody,
    confirmLabel: '삭제',
    action: 'danger',
    doneLabel: '삭제했어요',
  },
  'delete-note': {
    title: '노트를 지울까요?',
    icon: TrashIcon,
    body: DeleteNoteBody,
    confirmLabel: '노트 지우기',
    action: 'danger',
    doneLabel: '노트를 지웠어요',
  },
  'delete-goal': {
    title: '목표를 없앨까요?',
    icon: TargetIcon,
    body: DeleteGoalBody,
    confirmLabel: '목표 없애기',
    action: 'danger',
    doneLabel: '목표를 없앴어요',
  },
  'update-sharing': {
    title: '공개 범위를 바꿀까요?',
    icon: GlobeIcon,
    body: UpdateSharingBody,
    confirmLabel: '공개 범위 바꾸기',
    action: 'primary',
    doneLabel: '공개 범위를 바꿨어요',
  },
};

export const consequenceClass = css({ marginTop: '10px', fontSize: '13px', lineHeight: '[1.5]', color: 'text.muted' });

export type ActionOutcome = 'done' | 'declined' | 'failed' | 'closed';

export const actionOutcome = (message: Pick<ToolRequestMessage, 'status' | 'result'>): ActionOutcome => {
  if (message.status !== 'resolved') return 'closed';

  const failure = ToolFailureSchema.safeParse(message.result);
  if (!failure.success) return 'done';

  return failure.data.code === 'declined' ? 'declined' : 'failed';
};

export const ACTION_TAILS: Record<Exclude<ActionOutcome, 'done'>, string> = {
  declined: '그대로 뒀어요',
  failed: '처리하지 못했어요',
  closed: '확인하지 않아 닫혔어요',
};

export const actionTail = (outcome: ActionOutcome, card: Pick<ActionCard, 'doneLabel'>): string =>
  outcome === 'done' ? card.doneLabel : ACTION_TAILS[outcome];
