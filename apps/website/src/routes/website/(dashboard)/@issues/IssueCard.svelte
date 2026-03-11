<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import CalendarIcon from '~icons/lucide/calendar';
  import LinkIcon from '~icons/lucide/link';
  import IssuePriorityIcon from './IssuePriorityIcon.svelte';
  import IssueStatusIcon from './IssueStatusIcon.svelte';
  import type { IssuePriority, IssueStatus } from './constants';

  type Props = {
    issue: {
      id: string;
      content: string;
      status: string;
      priority: string;
      dueAt?: string | null;
      entities: readonly { id: string; slug: string; node: { __typename: string; title?: string; name?: string } }[];
    };
    ondragstart?: (e: DragEvent) => void;
    ondragend?: (e: DragEvent) => void;
    ondragenter?: () => void;
    onclick?: () => void;
  };

  let { issue, ondragstart, ondragend, ondragenter, onclick }: Props = $props();

  const isDone = $derived(issue.status === 'RESOLVED' || issue.status === 'CLOSED');

  const formattedDueAt = $derived.by(() => {
    if (!issue.dueAt) return null;
    const d = new Date(issue.dueAt);
    return `${d.getMonth() + 1}/${d.getDate()}`;
  });

  const isOverdue = $derived.by(() => {
    if (!issue.dueAt || isDone) return false;
    return dayjs(issue.dueAt).isBefore(dayjs(), 'day');
  });
</script>

<div
  class={css({
    backgroundColor: 'surface.default',
    borderRadius: '8px',
    paddingX: '12px',
    paddingY: '10px',
    cursor: 'pointer',
    transition: 'common',
    opacity: isDone ? '60' : '100',
    _hover: { backgroundColor: 'surface.muted' },
  })}
  draggable="true"
  onclick={(e) => {
    const target = e.target as HTMLElement;
    if (!target.closest('a')) {
      onclick?.();
    }
  }}
  {ondragend}
  {ondragenter}
  ondragstart={(e) => {
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', issue.id);

      const target = e.currentTarget as HTMLElement;
      const rect = target.getBoundingClientRect();
      const ghost = document.createElement('div');
      const cloned = target.cloneNode(true) as HTMLElement;
      cloned.style.pointerEvents = 'none';
      cloned.style.opacity = '0.85';
      cloned.style.width = '100%';
      ghost.append(cloned);
      ghost.style.position = 'absolute';
      ghost.style.width = `${rect.width}px`;
      ghost.style.top = '-1000px';
      ghost.style.left = '-1000px';
      document.body.append(ghost);
      e.dataTransfer.setDragImage(ghost, e.clientX - rect.left, e.clientY - rect.top);
      setTimeout(() => ghost.remove());
    }
    ondragstart?.(e);
  }}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onclick?.();
    }
  }}
  role="button"
  tabindex="0"
>
  <!-- 첫 번째 행: priority + status + content -->
  <div class={flex({ alignItems: 'flex-start', gap: '6px' })}>
    <div class={css({ flexShrink: '0', marginTop: '2px' })}>
      <IssuePriorityIcon priority={issue.priority as IssuePriority} />
    </div>
    <div class={css({ flexShrink: '0', marginTop: '1px' })}>
      <IssueStatusIcon status={issue.status as IssueStatus} />
    </div>

    <p
      class={css({
        fontSize: '13px',
        fontWeight: 'medium',
        color: isDone ? 'text.disabled' : 'text.default',
        lineClamp: '2',
        wordBreak: 'break-word',
        textDecoration: isDone ? 'line-through' : 'none',
      })}
    >
      {issue.content || '(내용 없음)'}
    </p>
  </div>

  <!-- 두 번째 행: 메타 정보 (dueAt, entities) -->
  {#if formattedDueAt || issue.entities.length > 0}
    <div class={flex({ gap: '8px', alignItems: 'center', marginTop: '6px', marginLeft: '36px' })}>
      {#if formattedDueAt}
        <div
          class={flex({
            alignItems: 'center',
            gap: '3px',
            fontSize: '11px',
            lineHeight: '[1]',
            fontWeight: 'medium',
            color: isOverdue ? 'accent.danger.default' : 'text.faint',
          })}
        >
          <Icon icon={CalendarIcon} size={12} />
          {formattedDueAt}
        </div>
      {/if}

      {#if issue.entities.length > 0}
        <div class={flex({ alignItems: 'center', gap: '3px', fontSize: '11px', lineHeight: '[1]', color: 'text.faint' })}>
          <Icon icon={LinkIcon} size={12} />
          <span>{issue.entities.length}</span>
        </div>
      {/if}
    </div>
  {/if}
</div>
