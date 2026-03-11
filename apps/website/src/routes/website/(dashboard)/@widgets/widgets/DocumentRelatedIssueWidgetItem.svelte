<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, Calendar, Icon, Menu, MenuItem, Popover } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import CalendarIcon from '~icons/lucide/calendar';
  import CheckIcon from '~icons/lucide/check';
  import XIcon from '~icons/lucide/x';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { getPriorityMeta, getStatusMeta, ISSUE_PRIORITIES, ISSUE_STATUSES } from '../../@issues/constants';
  import IssuePriorityIcon from '../../@issues/IssuePriorityIcon.svelte';
  import IssueStatusIcon from '../../@issues/IssueStatusIcon.svelte';
  import type { DocumentRelatedIssueWidgetItem_issue$key } from '$mearie';
  import type { IssuePriority, IssueStatus } from '../../@issues/constants';

  type Props = {
    issue$key: DocumentRelatedIssueWidgetItem_issue$key;
    palette?: boolean;
  };

  let { issue$key, palette = false }: Props = $props();

  const issue = createFragment(
    graphql(`
      fragment DocumentRelatedIssueWidgetItem_issue on Issue {
        id
        content
        status
        priority
        dueAt
        createdAt
      }
    `),
    () => issue$key,
  );

  const [updateIssue] = createMutation(
    graphql(`
      mutation DocumentRelatedIssueWidgetItem_UpdateIssue_Mutation($input: UpdateIssueInput!) {
        updateIssue(input: $input) {
          id
          content
          status
          priority
          dueAt
          updatedAt
        }
      }
    `),
  );

  const [deleteIssue] = createMutation(
    graphql(`
      mutation DocumentRelatedIssueWidgetItem_DeleteIssue_Mutation($input: DeleteIssueInput!) {
        deleteIssue(input: $input) {
          id
        }
      }
    `),
  );

  let editing = $state(false);
  let content = $state('');
  let contentUpdateTimeout: ReturnType<typeof setTimeout> | null = null;
  let contentInitialized = $state(false);
  let textareaEl = $state<HTMLTextAreaElement>();

  $effect(() => {
    if (!contentInitialized) {
      content = issue.data.content;
      contentInitialized = true;
    }
  });

  $effect(() => {
    if (textareaEl) {
      textareaEl.focus();
      requestAnimationFrame(() => {
        if (textareaEl) adjustHeight(textareaEl);
      });
    }
  });

  $effect(() => {
    if (editing) {
      return pushEscapeHandler(() => {
        editing = false;
        return true;
      });
    }

    if (contentUpdateTimeout) {
      clearTimeout(contentUpdateTimeout);
      contentUpdateTimeout = null;
      updateIssue({ input: { issueId: issue.data.id, content } });
    }
  });

  const handleContentInput = () => {
    if (contentUpdateTimeout) clearTimeout(contentUpdateTimeout);
    contentUpdateTimeout = setTimeout(async () => {
      await updateIssue({ input: { issueId: issue.data.id, content } });
      mixpanel.track('update_issue');
    }, 300);
  };

  const handleStatusChange = async (status: string) => {
    await updateIssue({ input: { issueId: issue.data.id, status: status as 'OPEN' | 'IN_PROGRESS' | 'RESOLVED' | 'CLOSED' } });
    mixpanel.track('update_issue');
    cache.invalidate({ __typename: 'Query', $field: 'issues' });
  };

  const handlePriorityChange = async (priority: string) => {
    await updateIssue({
      input: { issueId: issue.data.id, priority: priority as 'NONE' | 'LOW' | 'MEDIUM' | 'HIGH' | 'URGENT' },
    });
    mixpanel.track('update_issue');
    cache.invalidate({ __typename: 'Query', $field: 'issues' });
  };

  const handleDueAtChange = async (date: Date) => {
    await updateIssue({ input: { issueId: issue.data.id, dueAt: date.toISOString() } });
    mixpanel.track('update_issue');
    cache.invalidate({ __typename: 'Query', $field: 'issues' });
  };

  const handleClearDueAt = async () => {
    await updateIssue({ input: { issueId: issue.data.id, dueAt: null } });
    mixpanel.track('update_issue');
    cache.invalidate({ __typename: 'Query', $field: 'issues' });
  };

  const handleDelete = () => {
    Dialog.confirm({
      title: '할 일 삭제',
      message: '이 할 일을 삭제하시겠어요?',
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        await deleteIssue({ input: { issueId: issue.data.id } });
        mixpanel.track('delete_issue');
        cache.invalidate({ __typename: 'Query', $field: 'issues' });
        editing = false;
      },
    });
  };

  const handleSave = () => {
    if (contentUpdateTimeout) {
      clearTimeout(contentUpdateTimeout);
      contentUpdateTimeout = null;
      updateIssue({ input: { issueId: issue.data.id, content } });
    }
    editing = false;
  };

  let skipOutsideCheck = false;

  const handleClickOutside = (e: MouseEvent) => {
    if (skipOutsideCheck) {
      skipOutsideCheck = false;
      return;
    }
    const target = e.target as HTMLElement;
    if (
      target.closest('[role="menu"]') ||
      target.closest('[role="dialog"]') ||
      target.closest('[role="listbox"]') ||
      target.closest('[data-portal]')
    )
      return;
    if (!target.isConnected || !target.closest(`[data-widget-issue-id="${issue.data.id}"]`)) {
      editing = false;
    }
  };

  const adjustHeight = (el: HTMLTextAreaElement) => {
    el.style.height = 'auto';
    el.style.height = `${el.scrollHeight}px`;
  };

  const isDone = $derived(issue.data.status === 'RESOLVED' || issue.data.status === 'CLOSED');

  const isOverdue = $derived.by(() => {
    if (!issue.data.dueAt || isDone) return false;
    return dayjs(issue.data.dueAt as string).isBefore(dayjs(), 'day');
  });

  const formatDate = (date: string) => {
    const d = new Date(date);
    return `${d.getMonth() + 1}월 ${d.getDate()}일`;
  };

  const formattedDueAt = $derived.by(() => {
    if (!issue.data.dueAt) return null;
    const d = new Date(issue.data.dueAt as string);
    return `${d.getMonth() + 1}/${d.getDate()}`;
  });

  const chipFormattedDueAt = $derived(issue.data.dueAt ? formatDate(issue.data.dueAt as string) : '');

  const chipStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '5px',
    backgroundColor: 'surface.default',
    borderRadius: '6px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
    paddingX: '8px',
    paddingY: '2px',
    boxShadow: 'small',
    cursor: 'pointer',
    transition: 'common',
    userSelect: 'none',
    _hover: { backgroundColor: 'surface.subtle' },
  });
</script>

<svelte:window onclick={editing ? handleClickOutside : undefined} />

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class={css({
    position: 'relative',
    borderRadius: '8px',
    borderWidth: '1px',
    borderColor: editing ? 'border.strong' : 'border.subtle',
    backgroundColor: 'surface.default',
    transition: 'common',
    opacity: isDone && !editing ? '60' : '100',
    _hover: palette ? undefined : { borderColor: 'border.strong' },
  })}
  data-widget-issue-id={issue.data.id}
  onclick={(e) => {
    if (!editing && !palette && !(e.target as HTMLElement).closest('button')) {
      editing = true;
      skipOutsideCheck = true;
    }
  }}
  onkeydown={(e) => {
    if (!editing && !palette && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault();
      editing = true;
      skipOutsideCheck = true;
    }
  }}
  role={palette ? undefined : 'button'}
  tabindex={palette ? undefined : editing ? -1 : 0}
>
  {#if editing && !palette}
    <!-- Edit mode -->
    <div class={flex({ flexDirection: 'column', padding: '10px', gap: '8px', maxHeight: '300px', overflowY: 'auto' })}>
      <!-- Chips -->
      <div class={flex({ flexWrap: 'wrap', alignItems: 'center', gap: '6px' })}>
        <Menu style={chipStyle} disableAutoUpdate listStyle={css.raw({ minWidth: '[initial]' })} offset={4} placement="bottom-start">
          {#snippet button()}
            <IssueStatusIcon size={14} status={issue.data.status as IssueStatus} />
            <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
              {getStatusMeta(issue.data.status as IssueStatus).label}
            </span>
          {/snippet}
          {#snippet children({ close })}
            {#each ISSUE_STATUSES as s (s.value)}
              <MenuItem
                onclick={async () => {
                  await handleStatusChange(s.value);
                  close();
                }}
              >
                <div class={flex({ alignItems: 'center', justifyContent: 'space-between', gap: '20px', width: 'full' })}>
                  <div class={flex({ alignItems: 'center', gap: '6px' })}>
                    <IssueStatusIcon size={14} status={s.value} />
                    <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>{s.label}</span>
                  </div>
                  {#if issue.data.status === s.value}
                    <Icon style={css.raw({ color: 'text.subtle' })} icon={CheckIcon} size={14} />
                  {/if}
                </div>
              </MenuItem>
            {/each}
          {/snippet}
        </Menu>

        <Menu style={chipStyle} disableAutoUpdate listStyle={css.raw({ minWidth: '[initial]' })} offset={4} placement="bottom-start">
          {#snippet button()}
            <IssuePriorityIcon priority={issue.data.priority as IssuePriority} size={14} />
            <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
              {getPriorityMeta(issue.data.priority as IssuePriority).label}
            </span>
          {/snippet}
          {#snippet children({ close })}
            {#each ISSUE_PRIORITIES as p (p.value)}
              <MenuItem
                onclick={async () => {
                  await handlePriorityChange(p.value);
                  close();
                }}
              >
                <div class={flex({ alignItems: 'center', justifyContent: 'space-between', gap: '20px', width: 'full' })}>
                  <div class={flex({ alignItems: 'center', gap: '6px' })}>
                    <IssuePriorityIcon priority={p.value} size={14} />
                    <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>{p.label}</span>
                  </div>
                  {#if issue.data.priority === p.value}
                    <Icon style={css.raw({ color: 'text.subtle' })} icon={CheckIcon} size={14} />
                  {/if}
                </div>
              </MenuItem>
            {/each}
          {/snippet}
        </Menu>

        <Popover
          style={chipStyle}
          contentStyle={css.raw({ paddingX: '0', paddingY: '0', transformOrigin: 'top left' })}
          offset={4}
          placement="bottom-start"
        >
          {#snippet trigger()}
            <Icon style={css.raw({ color: 'text.faint' })} icon={CalendarIcon} size={14} />
            <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
              {issue.data.dueAt ? chipFormattedDueAt : '마감일'}
            </span>
            {#if issue.data.dueAt}
              <button
                class={center({
                  position: 'relative',
                  zIndex: '1',
                  size: '16px',
                  borderRadius: 'full',
                  color: 'text.faint',
                  cursor: 'pointer',
                  _hover: { color: 'text.default' },
                })}
                onclick={(e) => {
                  e.stopPropagation();
                  handleClearDueAt();
                }}
                type="button"
              >
                <Icon icon={XIcon} size={10} />
              </button>
            {/if}
          {/snippet}
          {#snippet children({ close })}
            <Calendar
              onchange={(d) => {
                handleDueAtChange(d);
                close();
              }}
              value={issue.data.dueAt ? new Date(issue.data.dueAt as string) : undefined}
            />
          {/snippet}
        </Popover>
      </div>

      <!-- Content -->
      <textarea
        bind:this={textareaEl}
        class={css({
          width: 'full',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.default',
          resize: 'none',
          minHeight: '36px',
        })}
        oninput={(e) => {
          adjustHeight(e.currentTarget);
          handleContentInput();
        }}
        placeholder="할 일 내용을 입력하세요"
        bind:value={content}
      ></textarea>

      <!-- Footer -->
      <div
        class={flex({
          justifyContent: 'flex-end',
          gap: '6px',
          paddingTop: '8px',
          borderTopWidth: '1px',
          borderColor: 'border.subtle',
        })}
      >
        <Button onclick={handleDelete} size="sm" variant="secondary">삭제</Button>
        <Button onclick={handleSave} size="sm">저장</Button>
      </div>
    </div>
  {:else}
    <!-- View mode (also palette mode) -->
    <div class={flex({ flexDirection: 'column', padding: '10px', cursor: palette ? 'default' : 'pointer', userSelect: 'none' })}>
      <div class={flex({ alignItems: 'flex-start', gap: '6px' })}>
        <div class={css({ flexShrink: '0', marginTop: '2px' })}>
          <IssuePriorityIcon priority={issue.data.priority as IssuePriority} />
        </div>
        <div class={css({ flexShrink: '0', marginTop: '1px' })}>
          <IssueStatusIcon status={issue.data.status as IssueStatus} />
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
          {issue.data.content || '(내용 없음)'}
        </p>
      </div>

      {#if formattedDueAt}
        <div
          class={flex({
            alignItems: 'center',
            gap: '3px',
            marginTop: '6px',
            marginLeft: '36px',
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
    </div>
  {/if}
</div>
