<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, Calendar, Icon, Menu, MenuItem, Modal, Popover } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import CalendarIcon from '~icons/lucide/calendar';
  import CheckIcon from '~icons/lucide/check';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import PlusIcon from '~icons/lucide/plus';
  import XIcon from '~icons/lucide/x';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { getPriorityMeta, getStatusMeta, ISSUE_PRIORITIES, ISSUE_STATUSES } from './constants';
  import IssueEntitySearchModal from './IssueEntitySearchModal.svelte';
  import IssuePriorityIcon from './IssuePriorityIcon.svelte';
  import IssueStatusIcon from './IssueStatusIcon.svelte';
  import type { IssuePriority, IssueStatus } from './constants';

  type Props = {
    open: boolean;
    issueId: string;
    onclose: () => void;
  };

  let { open, issueId, onclose }: Props = $props();

  const app = getAppContext();
  const siteId = $derived(app.preference.current.currentSiteId ?? '');

  const issueQuery = createQuery(
    graphql(`
      query IssueDetailModal_Query($issueId: ID!) {
        issue(issueId: $issueId) {
          id
          content
          status
          priority
          dueAt
          createdAt
          updatedAt
          entities {
            id
            slug
            node {
              __typename
              ... on Document {
                id
                title
              }
              ... on Folder {
                id
                name
              }
            }
          }
        }
      }
    `),
    () => ({ issueId }),
  );

  const [updateIssue] = createMutation(
    graphql(`
      mutation IssueDetailModal_UpdateIssue_Mutation($input: UpdateIssueInput!) {
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
      mutation IssueDetailModal_DeleteIssue_Mutation($input: DeleteIssueInput!) {
        deleteIssue(input: $input) {
          id
        }
      }
    `),
  );

  const [removeIssueEntity] = createMutation(
    graphql(`
      mutation IssueDetailModal_RemoveIssueEntity_Mutation($input: RemoveIssueEntityInput!) {
        removeIssueEntity(input: $input) {
          id
        }
      }
    `),
  );

  let content = $state('');
  let contentUpdateTimeout: ReturnType<typeof setTimeout> | null = null;
  let searchModalOpen = $state(false);
  let textareaEl = $state<HTMLTextAreaElement>();

  let contentInitialized = $state(false);

  $effect(() => {
    if (issueQuery.data?.issue && !contentInitialized) {
      content = issueQuery.data.issue.content;
      contentInitialized = true;
      if (textareaEl) {
        textareaEl.focus();
        requestAnimationFrame(() => {
          if (textareaEl) adjustHeight(textareaEl);
        });
      }
    }
  });

  $effect(() => {
    if (!open) {
      contentInitialized = false;
    }
  });

  const handleContentInput = () => {
    if (contentUpdateTimeout) clearTimeout(contentUpdateTimeout);
    contentUpdateTimeout = setTimeout(() => {
      updateIssue({ input: { issueId, content } });
      mixpanel.track('update_issue');
    }, 300);
  };

  const handleStatusChange = async (status: string) => {
    await updateIssue({ input: { issueId, status: status as 'OPEN' | 'IN_PROGRESS' | 'RESOLVED' | 'CLOSED' } });
    mixpanel.track('update_issue');
    cache.invalidate({ __typename: 'Query', $field: 'issues', $args: { siteId } });
  };

  const handlePriorityChange = async (priority: string) => {
    await updateIssue({ input: { issueId, priority: priority as 'NONE' | 'LOW' | 'MEDIUM' | 'HIGH' | 'URGENT' } });
    mixpanel.track('update_issue');
    cache.invalidate({ __typename: 'Query', $field: 'issues', $args: { siteId } });
  };

  const handleDueAtChange = async (date: Date) => {
    await updateIssue({ input: { issueId, dueAt: date.toISOString() } });
    mixpanel.track('update_issue');
    cache.invalidate({ __typename: 'Query', $field: 'issues', $args: { siteId } });
  };

  const handleClearDueAt = async () => {
    await updateIssue({ input: { issueId, dueAt: null } });
    mixpanel.track('update_issue');
    cache.invalidate({ __typename: 'Query', $field: 'issues', $args: { siteId } });
  };

  const handleSave = () => {
    if (contentUpdateTimeout) {
      clearTimeout(contentUpdateTimeout);
      contentUpdateTimeout = null;
      updateIssue({ input: { issueId, content } });
      cache.invalidate({ __typename: 'Query', $field: 'issues', $args: { siteId } });
    }
    onclose();
  };

  const handleDelete = () => {
    Dialog.confirm({
      title: '할 일 삭제',
      message: '이 할 일을 삭제하시겠어요?',
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        await deleteIssue({ input: { issueId } });
        mixpanel.track('delete_issue');
        cache.invalidate({ __typename: 'Query', $field: 'issues', $args: { siteId } });
        onclose();
      },
    });
  };

  const handleRemoveEntity = async (entityId: string) => {
    await removeIssueEntity({ input: { issueId, entityId } });
    cache.invalidate({ __typename: 'Query', $field: 'issues', $args: { siteId } });
    cache.invalidate({ __typename: 'Entity', id: entityId, $field: 'issues' });
  };

  const adjustHeight = (el: HTMLTextAreaElement) => {
    el.style.height = 'auto';
    el.style.height = `${el.scrollHeight}px`;
  };

  const issue = $derived(issueQuery.data?.issue);
  const existingEntityIds = $derived(issue?.entities.map((e) => e.id) ?? []);

  const formatDate = (date: string) => {
    const d = new Date(date);
    return `${d.getMonth() + 1}월 ${d.getDate()}일`;
  };

  const formatRelativeTime = (date: string) => {
    const now = Date.now();
    const diff = now - new Date(date).getTime();
    const minutes = Math.floor(diff / 60_000);
    if (minutes < 1) return '방금 전';
    if (minutes < 60) return `${minutes}분 전`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}시간 전`;
    const days = Math.floor(hours / 24);
    return `${days}일 전`;
  };

  const formattedDueAt = $derived(issue?.dueAt ? formatDate(issue.dueAt as string) : '');

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

<Modal style={css.raw({ width: '420px', maxWidth: '420px', maxHeight: '600px' })} loading={!issue} {onclose} {open}>
  {#if issue}
    <!-- Body -->
    <div
      class={flex({
        flexDirection: 'column',
        flexGrow: '1',
        overflowY: 'auto',
        paddingX: '16px',
        paddingTop: '16px',
        paddingBottom: '10px',
        gap: '16px',
      })}
    >
      <!-- Chips -->
      <div class={flex({ flexWrap: 'wrap', alignItems: 'center', gap: '6px' })}>
        <Menu style={chipStyle} disableAutoUpdate listStyle={css.raw({ minWidth: '[initial]' })} offset={4} placement="bottom-start">
          {#snippet button()}
            <IssueStatusIcon size={14} status={issue.status as IssueStatus} />
            <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
              {getStatusMeta(issue.status as IssueStatus).label}
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
                  {#if issue.status === s.value}
                    <Icon style={css.raw({ color: 'text.subtle' })} icon={CheckIcon} size={14} />
                  {/if}
                </div>
              </MenuItem>
            {/each}
          {/snippet}
        </Menu>

        <Menu style={chipStyle} disableAutoUpdate listStyle={css.raw({ minWidth: '[initial]' })} offset={4} placement="bottom-start">
          {#snippet button()}
            <IssuePriorityIcon priority={issue.priority as IssuePriority} size={13} />
            <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
              {getPriorityMeta(issue.priority as IssuePriority).label}
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
                    <IssuePriorityIcon priority={p.value} size={13} />
                    <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>{p.label}</span>
                  </div>
                  {#if issue.priority === p.value}
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
              {issue.dueAt ? formattedDueAt : '마감일'}
            </span>
            {#if issue.dueAt}
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
              value={issue.dueAt ? new Date(issue.dueAt as string) : undefined}
            />
          {/snippet}
        </Popover>

        <button
          class={center({
            marginLeft: 'auto',
            size: '28px',
            borderRadius: '6px',
            cursor: 'pointer',
            color: 'text.faint',
            _hover: { color: 'text.default' },
          })}
          onclick={onclose}
          type="button"
        >
          <Icon icon={XIcon} size={16} />
        </button>
      </div>

      <!-- Content -->
      <textarea
        bind:this={textareaEl}
        class={css({
          width: 'full',
          fontSize: '14px',
          fontWeight: 'medium',
          color: 'text.default',
          resize: 'none',
          minHeight: '60px',
        })}
        oninput={(e) => {
          adjustHeight(e.currentTarget);
          handleContentInput();
        }}
        placeholder="할 일 내용을 입력하세요"
        bind:value={content}
      ></textarea>

      <!-- Entities -->
      <div class={flex({ flexDirection: 'column', gap: '8px' })}>
        {#if issue.entities.length > 0}
          <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
            <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>연결된 항목</span>
            <button
              class={center({
                size: '20px',
                borderRadius: '4px',
                color: 'text.faint',
                cursor: 'pointer',
                _hover: { color: 'text.default' },
              })}
              onclick={() => (searchModalOpen = true)}
              type="button"
            >
              <Icon icon={PlusIcon} size={14} />
            </button>
          </div>
        {:else}
          <button
            class={flex({
              alignItems: 'center',
              gap: '4px',
              width: 'fit',
              fontSize: '12px',
              fontWeight: 'medium',
              color: 'text.faint',
              cursor: 'pointer',
              _hover: { color: 'text.default' },
            })}
            onclick={() => (searchModalOpen = true)}
            type="button"
          >
            <Icon icon={PlusIcon} size={14} />
            항목 연결하기
          </button>
        {/if}
        {#each issue.entities as entity (entity.id)}
          <div
            class={flex({
              alignItems: 'center',
              justifyContent: 'space-between',
              paddingX: '8px',
              paddingY: '6px',
              borderRadius: '6px',
              backgroundColor: 'surface.subtle',
            })}
          >
            {#if entity.node.__typename === 'Folder'}
              <div
                class={flex({
                  alignItems: 'center',
                  gap: '6px',
                  fontSize: '13px',
                  fontWeight: 'medium',
                  color: 'text.default',
                })}
              >
                <Icon icon={FolderIcon} size={14} />
                {entity.node.name || '(제목 없음)'}
              </div>
            {:else}
              <a
                class={flex({
                  alignItems: 'center',
                  gap: '6px',
                  fontSize: '13px',
                  fontWeight: 'medium',
                  color: 'text.default',
                  _hover: { color: 'accent.brand.default' },
                })}
                href={`/${entity.slug}`}
              >
                <Icon icon={FileIcon} size={14} />
                {(entity.node.__typename === 'Document' ? entity.node.title : '') || '(제목 없음)'}
              </a>
            {/if}
            <button
              class={center({
                size: '20px',
                borderRadius: '4px',
                color: 'text.faint',
                cursor: 'pointer',
                _hover: { color: 'text.default' },
              })}
              onclick={() => handleRemoveEntity(entity.id)}
              type="button"
            >
              <Icon icon={XIcon} size={12} />
            </button>
          </div>
        {/each}
      </div>
    </div>

    <!-- Footer -->
    <div
      class={flex({
        alignItems: 'center',
        justifyContent: 'space-between',
        paddingX: '16px',
        paddingY: '10px',
        borderTopWidth: '1px',
        borderColor: 'border.subtle',
        flexShrink: '0',
      })}
    >
      <div class={flex({ flexDirection: 'column', gap: '2px', fontSize: '11px', color: 'text.faint' })}>
        <span>생성: {formatDate(issue.createdAt as string)}</span>
        <span>수정: {formatRelativeTime(issue.updatedAt as string)}</span>
      </div>

      <div class={flex({ gap: '6px', alignItems: 'center' })}>
        <Button onclick={handleDelete} size="sm" variant="secondary">삭제</Button>
        <Button onclick={handleSave} size="sm">저장</Button>
      </div>
    </div>

    <IssueEntitySearchModal {existingEntityIds} {issueId} onclose={() => (searchModalOpen = false)} open={searchModalOpen} />
  {/if}
</Modal>
