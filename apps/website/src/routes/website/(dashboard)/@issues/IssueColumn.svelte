<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { getStatusMeta } from './constants';
  import IssueCard from './IssueCard.svelte';
  import IssueStatusIcon from './IssueStatusIcon.svelte';
  import type { IssueStatus } from './constants';

  type IssueData = {
    id: string;
    content: string;
    status: string;
    priority: string;
    dueAt?: string | null;
    entities: readonly { id: string; slug: string; node: { __typename: string; title?: string; name?: string } }[];
  };

  type Props = {
    status: IssueStatus;
    issues: readonly IssueData[];
    draggingStatus?: string | null;
    onstatuschange?: (issueId: string, newStatus: string) => void;
    onissueclick?: (issueId: string) => void;
    ondragstart?: (status: string) => void;
    ondragend?: () => void;
  };

  let {
    status,
    issues,
    draggingStatus = null,
    onstatuschange,
    onissueclick,
    ondragstart: onDragStart,
    ondragend: onDragEnd,
  }: Props = $props();

  const meta = $derived(getStatusMeta(status));
  const columnColor = $derived(token(`colors.${meta.colorToken}`));

  let dragCounter = $state(0);

  const isValidTarget = $derived(draggingStatus !== null && draggingStatus !== status);
  const isHovering = $derived(dragCounter > 0 && isValidTarget);

  const dropZoneShadow = $derived(isHovering ? `inset 0 0 0 2px ${columnColor}` : undefined);
</script>

<div
  style:box-shadow={dropZoneShadow}
  class={css({
    position: 'relative',
    display: 'flex',
    flexDirection: 'column',
    flex: '1',
    minWidth: '220px',
    height: 'full',
    backgroundColor: 'surface.subtle',
    borderRadius: '10px',
    padding: '10px',
    gap: '8px',
    overflow: 'hidden',
    transition: '[background-color 150ms ease, box-shadow 200ms ease]',
  })}
  ondragenter={(e) => {
    e.preventDefault();
    dragCounter++;
  }}
  ondragleave={() => {
    dragCounter--;
  }}
  ondragover={(e) => {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = isValidTarget ? 'move' : 'none';
    }
  }}
  ondrop={(e) => {
    e.preventDefault();
    dragCounter = 0;
    const issueId = e.dataTransfer?.getData('text/plain');
    if (issueId) {
      onstatuschange?.(issueId, status);
    }
  }}
  role="list"
>
  <div class={flex({ alignItems: 'center', gap: '6px', paddingX: '4px', paddingY: '4px', flexShrink: '0' })}>
    <IssueStatusIcon {status} />
    <span class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.default' })}>{meta.label}</span>
    <span class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>{issues.length}</span>
  </div>

  <div
    class={flex({
      flexDirection: 'column',
      gap: '6px',
      flexGrow: '1',
      overflowY: 'auto',
    })}
  >
    {#each issues as issue (issue.id)}
      <IssueCard
        {issue}
        onclick={() => onissueclick?.(issue.id)}
        ondragend={() => onDragEnd?.()}
        ondragstart={(e) => {
          onDragStart?.(status);
          if (e.dataTransfer) {
            e.dataTransfer.setData('text/plain', issue.id);
          }
        }}
      />
    {/each}
  </div>
</div>
