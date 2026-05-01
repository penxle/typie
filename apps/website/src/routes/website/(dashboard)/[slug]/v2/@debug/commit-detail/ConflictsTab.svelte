<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import dayjs from 'dayjs';
  import relativeTime from 'dayjs/plugin/relativeTime';
  import { graphql } from '$mearie';
  import type { DocumentEditorV2_Debug_ConflictsTab_commit$key } from '$mearie';

  dayjs.extend(relativeTime);

  type Props = {
    commit$key: DocumentEditorV2_Debug_ConflictsTab_commit$key;
  };

  let { commit$key }: Props = $props();

  const commit = createFragment(
    graphql(`
      fragment DocumentEditorV2_Debug_ConflictsTab_commit on DocumentCommit {
        id
        conflicts {
          id
          kind
          target
          baseValue
          autoResolvedBranch {
            id
          }
          branches {
            id
            commit {
              id
              hash
            }
            value
          }
          resolution {
            id
            value
            commit {
              id
              hash
            }
            createdAt
          }
          createdAt
        }
      }
    `),
    () => commit$key,
  );

  function fmt(value: unknown): string {
    return JSON.stringify(value, null, 2);
  }

  function fmtAbsolute(iso: string): string {
    return dayjs(iso).format('YYYY-MM-DD HH:mm:ss');
  }

  function fmtRelative(iso: string): string {
    return dayjs(iso).fromNow();
  }
</script>

{#snippet jsonInline(value: unknown)}
  <pre
    class={css({
      margin: '0',
      padding: '8px',
      borderWidth: '1px',
      borderColor: 'border.subtle',
      borderRadius: '4px',
      backgroundColor: 'surface.subtle',
      fontFamily: 'mono',
      fontSize: '11px',
      lineHeight: '[1.55]',
      whiteSpace: 'pre-wrap',
      wordBreak: 'break-all',
      color: 'text.default',
    })}>{fmt(value)}</pre>
{/snippet}

{#snippet fieldLabel(label: string)}
  <span
    class={css({
      fontFamily: 'ui',
      fontSize: '10px',
      fontWeight: 'semibold',
      letterSpacing: '[0.14em]',
      textTransform: 'uppercase',
      color: 'text.faint',
    })}
  >
    {label}
  </span>
{/snippet}

{#if commit.data.conflicts.length === 0}
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
      no conflicts
    </div>
    <div class={css({ fontSize: '12px', color: 'text.muted' })}>this commit did not require merge resolution</div>
  </div>
{:else}
  <div class={css({ display: 'flex', flexDirection: 'column', gap: '12px', fontFamily: 'ui', fontSize: '12px', lineHeight: '[1.55]' })}>
    {#each commit.data.conflicts as conflict (conflict.id)}
      <article
        class={css({
          borderWidth: '1px',
          borderColor: 'border.subtle',
          borderRadius: '6px',
          backgroundColor: 'surface.default',
          overflow: 'hidden',
        })}
      >
        <header
          class={css({
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '8px',
            paddingX: '12px',
            paddingY: '8px',
            borderBottomWidth: '1px',
            borderBottomColor: 'border.subtle',
            backgroundColor: 'surface.subtle',
          })}
        >
          <div class={css({ display: 'flex', alignItems: 'center', gap: '8px' })}>
            <span
              class={css({
                display: 'inline-flex',
                paddingX: '7px',
                paddingY: '2px',
                borderRadius: '[10px]',
                backgroundColor: 'palette.red/15',
                color: 'palette.red',
                fontFamily: 'ui',
                fontSize: '9px',
                fontWeight: 'semibold',
                letterSpacing: '[0.08em]',
                textTransform: 'uppercase',
              })}
            >
              {conflict.kind}
            </span>
            <span class={css({ fontFamily: 'mono', fontSize: '11px', color: 'text.muted' })}>
              {conflict.id.slice(-8)}
            </span>
          </div>
          <span class={css({ fontFamily: 'mono', fontSize: '10px', color: 'text.faint' })}>
            {fmtRelative(conflict.createdAt)}
          </span>
        </header>

        <div class={css({ padding: '12px', display: 'flex', flexDirection: 'column', gap: '12px' })}>
          <div class={css({ display: 'flex', flexDirection: 'column', gap: '5px' })}>
            {@render fieldLabel('target')}
            {@render jsonInline(conflict.target)}
          </div>

          {#if conflict.baseValue !== null && conflict.baseValue !== undefined}
            <div class={css({ display: 'flex', flexDirection: 'column', gap: '5px' })}>
              {@render fieldLabel('base value')}
              {@render jsonInline(conflict.baseValue)}
            </div>
          {/if}

          <div class={css({ display: 'flex', flexDirection: 'column', gap: '8px' })}>
            {@render fieldLabel(`branches (${conflict.branches.length})`)}
            <div class={css({ display: 'flex', flexDirection: 'column', gap: '8px' })}>
              {#each conflict.branches as branch (branch.id)}
                {@const isAuto = conflict.autoResolvedBranch?.id === branch.id}
                <div
                  class={css({
                    paddingLeft: '12px',
                    borderLeftWidth: '2px',
                    borderLeftColor: isAuto ? 'palette.yellow' : 'border.subtle',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '4px',
                  })}
                >
                  <div class={css({ display: 'flex', alignItems: 'center', gap: '8px' })}>
                    {#if isAuto}
                      <span
                        class={css({
                          display: 'inline-flex',
                          paddingX: '6px',
                          paddingY: '1px',
                          borderRadius: '[10px]',
                          backgroundColor: 'palette.yellow/20',
                          color: 'palette.yellow',
                          fontFamily: 'ui',
                          fontSize: '9px',
                          fontWeight: 'semibold',
                          letterSpacing: '[0.08em]',
                          textTransform: 'uppercase',
                        })}
                      >
                        auto
                      </span>
                    {/if}
                    <span class={css({ fontFamily: 'mono', fontSize: '11px', color: 'text.default' })}>
                      {branch.commit.hash.slice(0, 8)}
                    </span>
                  </div>
                  {@render jsonInline(branch.value)}
                </div>
              {/each}
            </div>
          </div>

          {#if conflict.resolution}
            {@const resolution = conflict.resolution}
            <div class={css({ display: 'flex', flexDirection: 'column', gap: '8px' })}>
              {@render fieldLabel('resolution')}
              <div
                class={css({
                  paddingLeft: '12px',
                  borderLeftWidth: '2px',
                  borderLeftColor: 'palette.green',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '4px',
                })}
              >
                <div class={css({ display: 'flex', alignItems: 'center', gap: '8px' })}>
                  <span class={css({ fontFamily: 'mono', fontSize: '11px', color: 'text.default' })}>
                    {resolution.commit.hash.slice(0, 8)}
                  </span>
                  <span class={css({ fontFamily: 'mono', fontSize: '10px', color: 'text.faint' })}>
                    {fmtAbsolute(resolution.createdAt)}
                  </span>
                </div>
                {@render jsonInline(resolution.value)}
              </div>
            </div>
          {/if}
        </div>
      </article>
    {/each}
  </div>
{/if}
