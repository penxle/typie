<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import CircleCheckBigIcon from '~icons/lucide/circle-check-big';
  import ExpandIcon from '~icons/lucide/expand';
  import Minimize2Icon from '~icons/lucide/minimize-2';
  import PlusIcon from '~icons/lucide/plus';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import Widget from '../Widget.svelte';
  import { getWidgetContext } from '../widget-context.svelte';
  import DocumentRelatedIssueWidgetItem from './DocumentRelatedIssueWidgetItem.svelte';

  type Props = {
    widgetId: string;
    data?: Record<string, unknown>;
  };

  let { widgetId, data = {} }: Props = $props();

  const app = getAppContext();
  const widgetContext = getWidgetContext();
  const { palette, document$key } = $derived(widgetContext.env);

  const relatedDocument = createFragment(
    graphql(`
      fragment Editor_Widget_DocumentRelatedIssueWidget_document on Document {
        id

        entity {
          id
          issues {
            id
            status
            priority
            createdAt
            ...DocumentRelatedIssueWidgetItem_issue
          }
        }
      }
    `),
    () => document$key,
  );

  const [createIssue] = createMutation(
    graphql(`
      mutation Editor_Widget_DocumentRelatedIssueWidget_CreateIssue_Mutation($input: CreateIssueInput!) {
        createIssue(input: $input) {
          id
        }
      }
    `),
  );

  let isExpanded = $state((data.isExpanded as boolean) ?? false);
  let isCollapsed = $state((data.isCollapsed as boolean) ?? false);

  const toggleExpanded = () => {
    isExpanded = !isExpanded;
    widgetContext.updateWidget?.(widgetId, { ...data, isExpanded, isCollapsed });
  };

  const toggleCollapse = () => {
    isCollapsed = !isCollapsed;
    widgetContext.updateWidget?.(widgetId, { ...data, isExpanded, isCollapsed });
  };

  const sortedIssues = $derived(relatedDocument.data?.entity.issues ?? []);

  const handleAddIssue = async () => {
    const siteId = app.preference.current.currentSiteId;
    const entityId = relatedDocument.data?.entity.id;
    if (!siteId || !entityId) return;

    await createIssue({
      input: {
        siteId,
        content: '',
        entityIds: [entityId],
      },
    });

    mixpanel.track('create_issue');
    cache.invalidate({ __typename: 'Entity', id: entityId, $field: 'issues' });
    cache.invalidate({ __typename: 'Query', $field: 'issues' });
  };
</script>

<Widget collapsed={isCollapsed} icon={CircleCheckBigIcon} noPadding title="할 일">
  {#snippet headerActions()}
    {#if !palette && !isCollapsed}
      <button
        class={center({
          height: '26px',
          borderRadius: '6px',
          paddingX: '6px',
          color: 'text.subtle',
          transition: 'common',
          _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
          cursor: 'pointer',
        })}
        onclick={(e) => {
          e.stopPropagation();
          handleAddIssue();
        }}
        onpointerdown={(e) => {
          e.stopPropagation();
        }}
        type="button"
      >
        <Icon icon={PlusIcon} size={14} />
      </button>
      <button
        class={center({
          height: '26px',
          borderRadius: '6px',
          paddingX: '6px',
          color: 'text.subtle',
          transition: 'common',
          _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
          cursor: 'pointer',
        })}
        onclick={(e) => {
          e.stopPropagation();
          toggleExpanded();
        }}
        onpointerdown={(e) => {
          e.stopPropagation();
        }}
        type="button"
        use:tooltip={{ message: isExpanded ? '크기 제한' : '크기 제한 해제', placement: 'top' }}
      >
        <Icon icon={isExpanded ? Minimize2Icon : ExpandIcon} size={14} />
      </button>
    {/if}
    <button
      class={cx(
        'group',
        flex({
          alignItems: 'center',
          height: '26px',
          borderRadius: '6px',
          paddingX: '6px',
          gap: '2px',
          color: 'text.subtle',
          cursor: 'pointer',
          _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
        }),
      )}
      onclick={toggleCollapse}
      type="button"
    >
      <Icon icon={isCollapsed ? ChevronDownIcon : ChevronUpIcon} size={14} />
    </button>
  {/snippet}

  <div
    class={flex({
      flexDirection: 'column',
      gap: '6px',
      maxHeight: isExpanded ? undefined : '400px',
      overflowY: 'auto',
      padding: '8px',
      paddingRight: '4px',
    })}
  >
    {#if sortedIssues.length === 0}
      <div
        class={flex({
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '12px',
          paddingY: '24px',
        })}
      >
        <div
          class={center({
            size: '48px',
            borderRadius: '12px',
            backgroundColor: 'surface.muted',
            color: 'text.faint',
          })}
        >
          <Icon icon={CircleCheckBigIcon} size={20} />
        </div>

        <p class={css({ fontSize: '12px', color: 'text.faint', textAlign: 'center' })}>할 일이 없습니다</p>
      </div>
    {:else}
      {#each sortedIssues as issue (issue.id)}
        <DocumentRelatedIssueWidgetItem issue$key={issue} {palette} />
      {/each}
    {/if}
  </div>
</Widget>
