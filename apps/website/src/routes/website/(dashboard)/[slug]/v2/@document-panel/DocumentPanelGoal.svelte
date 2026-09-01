<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { ProgressBar } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { dueStatus, goalColorState, pickGoalSource } from '$lib/goal';
  import { graphql } from '$mearie';
  import { getDayClock } from '../../../day-clock.svelte';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';
  import type { DocumentPanelV2_Goal_document$key } from '$mearie';

  type Props = {
    document$key: DocumentPanelV2_Goal_document$key;
    editor: Editor | undefined;
  };

  let { document$key, editor }: Props = $props();

  const app = getAppContext();
  const dayClock = getDayClock();

  const document = createFragment(
    graphql(`
      fragment DocumentPanelV2_Goal_document on Document {
        id
        characterCount

        entity {
          id

          goal {
            id
            targetCharacterCount
            dueAt
            createdAt
          }

          ancestors {
            id

            goal {
              id
              targetCharacterCount
              dueAt
              createdAt
            }

            node {
              __typename
              ... on Folder {
                id
                characterCount
              }
            }
          }
        }
      }
    `),
    () => document$key,
  );

  const active = $derived(pickGoalSource(document.data.entity, editor?.characterCounts.docWithWhitespace ?? document.data.characterCount));
</script>

{#if active}
  {@const state = goalColorState(active.current, active.goal.targetCharacterCount)}
  {@const today = dayClock.now}
  {@const due = active.goal.dueAt ? dayjs(active.goal.dueAt).kst() : null}

  <div class={flex({ flexDirection: 'column', gap: '6px' })}>
    <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
      <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>
        {active.isFolder ? '폴더 목표' : '목표'}
      </div>

      <button
        class={css({
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.faint',
          transition: 'common',
          _hover: { color: 'text.subtle' },
        })}
        onclick={() => {
          app.state.goalOpen = [active.entityId];
          mixpanel.track('open_goal_modal', { via: 'panel' });
        }}
        type="button"
      >
        자세히
      </button>
    </div>

    <ProgressBar progress={active.current / active.goal.targetCharacterCount} {state} />

    <div
      class={flex({
        justifyContent: 'space-between',
        flexWrap: 'wrap',
        columnGap: '8px',
        rowGap: '2px',
        fontSize: '13px',
        color: 'text.subtle',
      })}
    >
      <span class={css({ whiteSpace: 'nowrap' })}>{comma(active.current)} / {comma(active.goal.targetCharacterCount)}자</span>

      {#if due}
        {@const status = dueStatus(active.current, active.goal.targetCharacterCount, due, today, 'compact')}

        {#if status}
          <span
            class={css(status.warning ? { color: 'text.danger' } : { color: 'text.faint' }, { whiteSpace: 'nowrap', marginLeft: 'auto' })}
          >
            {status.label}
          </span>
        {/if}
      {/if}
    </div>
  </div>
{/if}
