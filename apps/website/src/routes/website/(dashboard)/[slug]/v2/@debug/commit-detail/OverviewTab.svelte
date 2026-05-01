<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import dayjs from 'dayjs';
  import relativeTime from 'dayjs/plugin/relativeTime';
  import { graphql } from '$mearie';
  import type { DocumentEditorV2_Debug_OverviewTab_commit$key } from '$mearie';

  dayjs.extend(relativeTime);

  type Props = {
    commit$key: DocumentEditorV2_Debug_OverviewTab_commit$key;
    onNavigate: (id: string) => void;
  };

  let { commit$key, onNavigate }: Props = $props();

  const commit = createFragment(
    graphql(`
      fragment DocumentEditorV2_Debug_OverviewTab_commit on DocumentCommit {
        id
        hash
        parent {
          id
          hash
        }
        secondParent {
          id
          hash
        }
        rootObject {
          id
          hash
        }
        device {
          id
          name
          isCurrent
        }
        user {
          id
        }
        meta
        committedAt
        pushedAt
      }
    `),
    () => commit$key,
  );

  function fmtAbsolute(iso: string): string {
    return dayjs(iso).format('YYYY-MM-DD HH:mm:ss');
  }

  function fmtRelative(iso: string): string {
    return dayjs(iso).fromNow();
  }
</script>

{#snippet sectionHeader(label: string)}
  <div class={css({ display: 'flex', alignItems: 'center', gap: '10px', marginBottom: '12px' })}>
    <span
      class={css({
        fontFamily: 'ui',
        fontSize: '10px',
        fontWeight: 'semibold',
        letterSpacing: '[0.14em]',
        textTransform: 'uppercase',
        color: 'text.muted',
      })}
    >
      {label}
    </span>
    <hr
      class={css({
        flexGrow: '1',
        borderTopWidth: '1px',
        borderTopColor: 'border.subtle',
        borderBottom: 'none',
        borderLeft: 'none',
        borderRight: 'none',
        margin: '0',
      })}
    />
  </div>
{/snippet}

{#snippet hashPill(hash: string, id: string)}
  <button
    class={css({
      cursor: 'pointer',
      display: 'inline-flex',
      alignItems: 'center',
      gap: '5px',
      paddingX: '8px',
      paddingY: '3px',
      borderWidth: '1px',
      borderColor: 'border.subtle',
      borderRadius: '4px',
      backgroundColor: 'surface.subtle',
      fontFamily: 'mono',
      fontSize: '11px',
      color: 'text.default',
      transition: '[background-color 100ms, border-color 100ms]',
      _hover: { backgroundColor: 'surface.muted', borderColor: 'border.default' },
    })}
    onclick={() => onNavigate(id)}
    type="button"
  >
    {hash.slice(0, 8)}
    <span class={css({ fontSize: '9px', color: 'text.faint' })}>↗</span>
  </button>
{/snippet}

<div class={css({ display: 'flex', flexDirection: 'column', gap: '24px', fontFamily: 'ui', fontSize: '12px', lineHeight: '[1.55]' })}>
  <section>
    {@render sectionHeader('Hash')}
    <div
      class={css({
        fontFamily: 'mono',
        fontSize: '13px',
        color: 'text.default',
        userSelect: 'text',
        wordBreak: 'break-all',
      })}
    >
      {commit.data.hash}
    </div>
  </section>

  <section>
    {@render sectionHeader('Lineage')}
    <dl
      class={css({
        margin: '0',
        display: 'grid',
        gridTemplateColumns: '[120px 1fr]',
        columnGap: '16px',
        rowGap: '10px',
        alignItems: 'center',
      })}
    >
      <dt class={css({ color: 'text.faint', fontSize: '11px' })}>parent</dt>
      <dd class={css({ margin: '0' })}>
        {#if commit.data.parent}
          {@const parent = commit.data.parent}
          {@render hashPill(parent.hash, parent.id)}
        {:else}
          <span class={css({ color: 'text.faint' })}>—</span>
        {/if}
      </dd>

      {#if commit.data.secondParent}
        {@const secondParent = commit.data.secondParent}
        <dt class={css({ color: 'text.faint', fontSize: '11px' })}>secondParent</dt>
        <dd class={css({ margin: '0' })}>
          {@render hashPill(secondParent.hash, secondParent.id)}
        </dd>
      {/if}

      <dt class={css({ color: 'text.faint', fontSize: '11px' })}>rootObject</dt>
      <dd class={css({ margin: '0', fontFamily: 'mono', fontSize: '11px', color: 'text.muted' })}>
        {commit.data.rootObject.hash.slice(0, 8)}
      </dd>
    </dl>
  </section>

  <div class={css({ display: 'grid', gridTemplateColumns: '[1fr 1fr]', columnGap: '32px', rowGap: '24px' })}>
    <section>
      {@render sectionHeader('Author')}
      <dl class={css({ margin: '0', display: 'flex', flexDirection: 'column', gap: '12px' })}>
        <div>
          <dt class={css({ color: 'text.faint', fontSize: '11px', marginBottom: '3px' })}>device</dt>
          <dd class={css({ margin: '0', display: 'flex', alignItems: 'center', gap: '8px', flexWrap: 'wrap' })}>
            {#if commit.data.device}
              <span class={css({ color: 'text.default' })}>{commit.data.device.name}</span>
              {#if commit.data.device.isCurrent}
                <span
                  class={css({
                    display: 'inline-flex',
                    paddingX: '6px',
                    paddingY: '1px',
                    borderRadius: '[10px]',
                    backgroundColor: 'palette.orange/15',
                    color: 'palette.orange',
                    fontSize: '9px',
                    fontWeight: 'semibold',
                    letterSpacing: '[0.08em]',
                    textTransform: 'uppercase',
                  })}
                >
                  this device
                </span>
              {/if}
            {:else}
              <span class={css({ color: 'text.faint' })}>—</span>
            {/if}
          </dd>
        </div>

        <div>
          <dt class={css({ color: 'text.faint', fontSize: '11px', marginBottom: '3px' })}>user</dt>
          <dd class={css({ margin: '0', fontFamily: 'mono', fontSize: '11px', color: 'text.default' })}>
            {commit.data.user?.id ?? '—'}
          </dd>
        </div>
      </dl>
    </section>

    <section>
      {@render sectionHeader('Timing')}
      <dl class={css({ margin: '0', display: 'flex', flexDirection: 'column', gap: '12px' })}>
        <div>
          <dt class={css({ color: 'text.faint', fontSize: '11px', marginBottom: '3px' })}>committed</dt>
          <dd class={css({ margin: '0', display: 'flex', flexDirection: 'column', gap: '1px' })}>
            <span class={css({ fontFamily: 'mono', fontSize: '12px', color: 'text.default' })}>
              {fmtAbsolute(commit.data.committedAt)}
            </span>
            <span class={css({ fontSize: '11px', color: 'text.faint' })}>
              {fmtRelative(commit.data.committedAt)}
            </span>
          </dd>
        </div>

        <div>
          <dt class={css({ color: 'text.faint', fontSize: '11px', marginBottom: '3px' })}>pushed</dt>
          <dd class={css({ margin: '0', display: 'flex', flexDirection: 'column', gap: '1px' })}>
            {#if commit.data.pushedAt}
              <span class={css({ fontFamily: 'mono', fontSize: '12px', color: 'text.default' })}>
                {fmtAbsolute(commit.data.pushedAt)}
              </span>
              <span class={css({ fontSize: '11px', color: 'text.faint' })}>
                {fmtRelative(commit.data.pushedAt)}
              </span>
            {:else}
              <span class={css({ color: 'text.faint', fontStyle: 'italic' })}>not pushed</span>
            {/if}
          </dd>
        </div>
      </dl>
    </section>
  </div>

  {#if commit.data.meta !== null && commit.data.meta !== undefined}
    <section>
      {@render sectionHeader('Meta')}
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
        })}>{JSON.stringify(commit.data.meta, null, 2)}</pre>
    </section>
  {/if}
</div>
