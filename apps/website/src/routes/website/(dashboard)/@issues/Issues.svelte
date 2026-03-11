<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import PlusIcon from '~icons/lucide/plus';
  import SearchIcon from '~icons/lucide/search';
  import { beforeNavigate } from '$app/navigation';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { ISSUE_STATUSES } from './constants';
  import IssueColumn from './IssueColumn.svelte';
  import IssueDetailModal from './IssueDetailModal.svelte';

  const app = getAppContext();

  const issuesQuery = createQuery(
    graphql(`
      query Issues_Query($siteId: ID!) {
        issues(siteId: $siteId) {
          id
          content
          status
          priority
          dueAt
          createdAt
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
    () => ({ siteId: app.preference.current.currentSiteId ?? '' }),
    () => ({ skip: !app.state.issuesOpen }),
  );

  const [createIssue] = createMutation(
    graphql(`
      mutation Issues_CreateIssue_Mutation($input: CreateIssueInput!) {
        createIssue(input: $input) {
          id
          content
          status
          priority
          dueAt
        }
      }
    `),
  );

  const [updateIssue] = createMutation(
    graphql(`
      mutation Issues_UpdateIssue_Mutation($input: UpdateIssueInput!) {
        updateIssue(input: $input) {
          id
          status
        }
      }
    `),
  );

  let selectedIssueId = $state<string | null>(null);
  let filterQuery = $state('');
  let draggingStatus = $state<string | null>(null);

  const issues = $derived(issuesQuery.data?.issues ?? []);

  const filteredIssues = $derived.by(() => {
    if (!filterQuery.trim()) return issues;
    const q = filterQuery.toLowerCase();
    return issues.filter((issue) => issue.content.toLowerCase().includes(q));
  });

  const issuesByStatus = $derived.by(() => {
    const grouped: Record<string, typeof filteredIssues> = {};
    for (const s of ISSUE_STATUSES) {
      grouped[s.value] = filteredIssues.filter((issue) => issue.status === s.value);
    }
    return grouped;
  });

  const handleCreateIssue = async () => {
    if (!app.preference.current.currentSiteId) return;
    const result = await createIssue({
      input: {
        siteId: app.preference.current.currentSiteId,
        content: '',
      },
    });
    mixpanel.track('create_issue');
    cache.invalidate({ __typename: 'Query', $field: 'issues', $args: { siteId: app.preference.current.currentSiteId ?? '' } });
    selectedIssueId = result.createIssue.id;
  };

  const handleStatusChange = async (issueId: string, newStatus: string) => {
    const issue = issues.find((i) => i.id === issueId);
    if (!issue || issue.status === newStatus) return;

    try {
      await updateIssue({
        input: { issueId, status: newStatus as 'OPEN' | 'IN_PROGRESS' | 'RESOLVED' | 'CLOSED' },
      });
      mixpanel.track('update_issue');
      cache.invalidate({ __typename: 'Query', $field: 'issues', $args: { siteId: app.preference.current.currentSiteId ?? '' } });
    } catch {
      Toast.error('상태 변경에 실패했습니다.');
    }
  };

  const close = () => {
    app.state.issuesOpen = false;
    selectedIssueId = null;
  };

  const handleKeyDown = (event: KeyboardEvent) => {
    const metaOrCtrlKeyOnly = (event.metaKey && !event.ctrlKey) || (event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey);

    if (metaOrCtrlKeyOnly && event.key === 'j') {
      event.preventDefault();
      app.state.issuesOpen = !app.state.issuesOpen;
      if (!app.state.issuesOpen) {
        selectedIssueId = null;
      }
    } else if (app.state.issuesOpen && event.key === 'Escape') {
      event.stopPropagation();
      if (selectedIssueId) {
        selectedIssueId = null;
        return;
      }
      close();
    }
  };

  beforeNavigate(() => {
    close();
  });

  $effect(() => {
    if (app.state.issuesOpen) {
      cache.invalidate({ __typename: 'Query', $field: 'issues', $args: { siteId: app.preference.current.currentSiteId ?? '' } });
    }
  });
</script>

<svelte:window onkeydown={handleKeyDown} />

<Modal
  style={css.raw({
    backgroundColor: 'transparent',
    maxWidth: 'full',
    height: 'full',
    border: 'none',
    boxShadow: '[none]',
    padding: '0',
  })}
  onclose={close}
  open={app.state.issuesOpen}
  overlayPadding={0}
>
  <div
    class={flex({
      position: 'relative',
      flexDirection: 'column',
      width: 'full',
      height: 'full',
    })}
  >
    <!-- Backdrop close -->
    <div class={css({ position: 'absolute', inset: '0', zIndex: '0' })} onclick={close} role="none"></div>

    <div
      class={flex({
        position: 'relative',
        zIndex: '1',
        flexDirection: 'column',
        maxWidth: '1200px',
        maxHeight: '800px',
        width: 'full',
        height: 'full',
        marginX: 'auto',
        marginY: 'auto',
        pointerEvents: 'none',
      })}
    >
      <!-- Header -->
      <div
        class={flex({
          alignItems: 'center',
          justifyContent: 'space-between',
          paddingX: '24px',
          paddingY: '16px',
          flexShrink: '0',
        })}
      >
        <div class={flex({ alignItems: 'center', gap: '8px', pointerEvents: 'auto', userSelect: 'none' })}>
          <h1 class={css({ fontSize: '18px', fontWeight: 'bold', color: 'text.bright' })}>할 일</h1>
          <kbd
            class={center({
              gap: '2px',
              borderRadius: '4px',
              paddingX: '6px',
              paddingTop: '3px',
              paddingBottom: '1px',
              fontFamily: 'mono',
              fontSize: '11px',
              fontWeight: 'medium',
              color: 'text.bright',
              backgroundColor: 'surface.dark/20',
            })}
          >
            <span>{navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}</span>
            {#if !navigator.platform.includes('Mac')}
              <span>+</span>
            {/if}
            <span>J</span>
          </kbd>
        </div>

        <div class={flex({ alignItems: 'center', gap: '8px', pointerEvents: 'auto' })}>
          <div
            class={flex({
              alignItems: 'center',
              gap: '6px',
              backgroundColor: 'surface.default/80',
              backdropFilter: 'auto',
              backdropBlur: '6px',
              borderRadius: '6px',
              paddingX: '10px',
              height: '34px',
              borderWidth: '1px',
              borderColor: 'border.subtle',
            })}
          >
            <Icon style={css.raw({ color: 'text.faint' })} icon={SearchIcon} size={14} />
            <input
              class={css({ fontSize: '13px', color: 'text.default', width: '160px' })}
              placeholder="검색..."
              bind:value={filterQuery}
            />
          </div>
          <Button onclick={handleCreateIssue} size="md">
            <Icon icon={PlusIcon} size={14} />
            <span class={css({ marginLeft: '2px' })}>새 할 일</span>
          </Button>
        </div>
      </div>

      <!-- Board -->
      <div
        class={flex({
          flexGrow: '1',
          overflow: 'hidden',
          pointerEvents: 'auto',
        })}
      >
        <div
          class={flex({
            flexGrow: '1',
            gap: '12px',
            paddingX: '24px',

            overflowX: 'auto',
            overflowY: 'hidden',
            height: 'full',
          })}
        >
          {#each ISSUE_STATUSES as statusMeta (statusMeta.value)}
            <IssueColumn
              {draggingStatus}
              issues={issuesByStatus[statusMeta.value] ?? []}
              ondragend={() => (draggingStatus = null)}
              ondragstart={(issueStatus) => (draggingStatus = issueStatus)}
              onissueclick={(id) => (selectedIssueId = id)}
              onstatuschange={handleStatusChange}
              status={statusMeta.value}
            />
          {/each}
        </div>
      </div>

      <!-- Footer -->
      <div
        class={css({
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          paddingX: '24px',
          paddingY: '16px',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.bright',
          flexShrink: '0',
          pointerEvents: 'auto',
          userSelect: 'none',
        })}
      >
        <span>드래그하여 상태 변경</span>
      </div>
    </div>
  </div>

  {#if selectedIssueId}
    <IssueDetailModal issueId={selectedIssueId} onclose={() => (selectedIssueId = null)} open={!!selectedIssueId} />
  {/if}
</Modal>
