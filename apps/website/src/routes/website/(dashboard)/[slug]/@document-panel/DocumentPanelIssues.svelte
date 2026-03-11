<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import CircleCheckBigIcon from '~icons/lucide/circle-check-big';
  import PlusIcon from '~icons/lucide/plus';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import DocumentPanelIssueItem from './DocumentPanelIssueItem.svelte';
  import type { DocumentPanel_Issues_entity$key } from '$mearie';

  type Props = {
    entity$key: DocumentPanel_Issues_entity$key;
  };

  let { entity$key }: Props = $props();

  const app = getAppContext();

  const entity = createFragment(
    graphql(`
      fragment DocumentPanel_Issues_entity on Entity {
        id
        issues {
          id
          status
          priority
          createdAt
          ...DocumentPanelIssueItem_issue
        }
      }
    `),
    () => entity$key,
  );

  const [createIssue] = createMutation(
    graphql(`
      mutation DocumentPanelIssues_CreateIssue_Mutation($input: CreateIssueInput!) {
        createIssue(input: $input) {
          id
        }
      }
    `),
  );

  let lastAddedIssueId = $state<string>();

  const sortedIssues = $derived(entity.data.issues);

  const handleAddIssue = async () => {
    const siteId = app.preference.current.currentSiteId;
    if (!siteId) return;

    const result = await createIssue({
      input: {
        siteId,
        content: '',
        entityIds: [entity.data.id],
      },
    });

    if (result?.createIssue?.id) {
      lastAddedIssueId = result.createIssue.id;
      mixpanel.track('create_issue');
      cache.invalidate({ __typename: 'Entity', id: entity.data.id, $field: 'issues' });
      cache.invalidate({ __typename: 'Query', $field: 'issues' });
    }
  };
</script>

<div class={flex({ flexDirection: 'column', flexGrow: '1', height: 'full', overflow: 'hidden' })}>
  <div
    class={flex({
      justifyContent: 'space-between',
      alignItems: 'center',
      height: '41px',
      paddingX: '20px',
      flexShrink: '0',
      borderBottomWidth: '1px',
      borderColor: 'surface.muted',
    })}
  >
    <div class={flex({ alignItems: 'center', gap: '6px', fontWeight: 'semibold' })}>
      <div class={css({ fontSize: '13px', color: 'text.subtle' })}>할 일</div>
      {#if sortedIssues.length > 0}
        <div
          class={css({
            fontSize: '11px',
            color: 'text.default',
            backgroundColor: 'surface.muted',
            paddingX: '6px',
            paddingY: '2px',
            borderRadius: '4px',
          })}
        >
          {sortedIssues.length}
        </div>
      {/if}
    </div>

    <button
      class={center({
        size: '20px',
        color: 'text.faint',
        transition: 'common',
        _hover: { color: 'text.subtle' },
        cursor: 'pointer',
      })}
      onclick={handleAddIssue}
      type="button"
      use:tooltip={{ message: '할 일 추가', placement: 'top' }}
    >
      <Icon icon={PlusIcon} size={14} />
    </button>
  </div>

  <div
    class={flex({
      flexDirection: 'column',
      gap: '6px',
      flexGrow: '1',
      overflowY: 'auto',
      paddingX: '8px',
      paddingTop: '8px',
      paddingBottom: '20px',
    })}
  >
    {#if sortedIssues.length === 0}
      <div
        class={flex({
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '20px',
          paddingY: '60px',
        })}
      >
        <div
          class={center({
            size: '64px',
            borderRadius: '16px',
            backgroundColor: 'surface.muted',
            color: 'text.faint',
          })}
        >
          <Icon icon={CircleCheckBigIcon} size={28} />
        </div>

        <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '8px' })}>
          <p class={css({ fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>할 일이 없습니다</p>
        </div>

        <Button onclick={handleAddIssue} size="sm" variant="secondary">새 할 일</Button>
      </div>
    {:else}
      {#each sortedIssues as issue (issue.id)}
        <DocumentPanelIssueItem autoEdit={issue.id === lastAddedIssueId} issue$key={issue} />
      {/each}
    {/if}
  </div>
</div>
