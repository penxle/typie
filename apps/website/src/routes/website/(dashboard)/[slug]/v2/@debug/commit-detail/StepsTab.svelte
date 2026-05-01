<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { graphql } from '$mearie';
  import type { DocumentEditorV2_Debug_StepsTab_commit$key } from '$mearie';

  type Props = {
    commit$key: DocumentEditorV2_Debug_StepsTab_commit$key;
  };

  let { commit$key }: Props = $props();

  const commit = createFragment(
    graphql(`
      fragment DocumentEditorV2_Debug_StepsTab_commit on DocumentCommit {
        id
        steps
      }
    `),
    () => commit$key,
  );
</script>

{#if commit.data.steps === null || commit.data.steps === undefined}
  <div
    class={css({
      paddingY: '64px',
      paddingX: '24px',
      borderWidth: '1px',
      borderColor: 'border.subtle',
      borderStyle: 'dashed',
      borderRadius: '6px',
      textAlign: 'center',
      fontFamily: 'ui',
    })}
  >
    <div
      class={css({
        fontSize: '10px',
        fontWeight: 'semibold',
        letterSpacing: '[0.14em]',
        textTransform: 'uppercase',
        color: 'text.faint',
        marginBottom: '6px',
      })}
    >
      no steps
    </div>
    <div class={css({ fontSize: '12px', color: 'text.muted' })}>initial commit or non-step-bearing commit</div>
  </div>
{:else}
  <pre
    class={css({
      margin: '0',
      padding: '12px',
      borderWidth: '1px',
      borderColor: 'border.subtle',
      borderRadius: '6px',
      backgroundColor: 'surface.subtle',
      fontFamily: 'mono',
      fontSize: '11px',
      lineHeight: '[1.6]',
      whiteSpace: 'pre-wrap',
      wordBreak: 'break-all',
      color: 'text.default',
    })}>{JSON.stringify(commit.data.steps, null, 2)}</pre>
{/if}
