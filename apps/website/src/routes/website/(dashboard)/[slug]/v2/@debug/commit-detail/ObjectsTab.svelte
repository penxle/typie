<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { graphql } from '$mearie';
  import type { DocumentEditorV2_Debug_ObjectsTab_commit$key } from '$mearie';

  type Props = {
    commit$key: DocumentEditorV2_Debug_ObjectsTab_commit$key;
  };

  let { commit$key }: Props = $props();

  const commit = createFragment(
    graphql(`
      fragment DocumentEditorV2_Debug_ObjectsTab_commit on DocumentCommit {
        id
        rootObject {
          id
          hash
        }
        objects {
          id
          hash
          content
        }
      }
    `),
    () => commit$key,
  );

  const sortedObjects = $derived.by(() => {
    const rootHash = commit.data.rootObject.hash;
    const list = [...commit.data.objects];
    list.sort((a, b) => {
      if (a.hash === rootHash) return -1;
      if (b.hash === rootHash) return 1;
      return a.hash.localeCompare(b.hash);
    });
    return list;
  });

  function sizeHint(content: unknown): string {
    return `${JSON.stringify(content).length} chars`;
  }
</script>

<div class={css({ display: 'flex', flexDirection: 'column', gap: '14px', fontFamily: 'ui', fontSize: '12px', lineHeight: '[1.55]' })}>
  <div
    class={css({
      paddingBottom: '10px',
      borderBottomWidth: '1px',
      borderBottomColor: 'border.subtle',
      fontSize: '11px',
      color: 'text.muted',
    })}
  >
    {commit.data.objects.length} reachable objects
  </div>

  <ul class={css({ listStyle: 'none', padding: '0', margin: '0', display: 'flex', flexDirection: 'column', gap: '4px' })}>
    {#each sortedObjects as obj (obj.id)}
      {@const isRoot = obj.hash === commit.data.rootObject.hash}
      <li>
        <details
          class={css({
            borderWidth: '1px',
            borderColor: 'border.subtle',
            borderRadius: '4px',
            backgroundColor: 'surface.default',
            transition: '[border-color 100ms]',
            _hover: { borderColor: 'border.default' },
            '&[open]': { borderColor: 'border.default' },
          })}
        >
          <summary
            class={css({
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: '10px',
              paddingX: '12px',
              paddingY: '8px',
              listStyle: 'none',
              '&::-webkit-details-marker': { display: 'none' },
            })}
          >
            {#if isRoot}
              <span
                class={css({
                  display: 'inline-flex',
                  paddingX: '6px',
                  paddingY: '1px',
                  borderRadius: '[10px]',
                  backgroundColor: 'palette.orange/15',
                  color: 'palette.orange',
                  fontFamily: 'ui',
                  fontSize: '9px',
                  fontWeight: 'semibold',
                  letterSpacing: '[0.08em]',
                  textTransform: 'uppercase',
                })}
              >
                root
              </span>
            {/if}
            <span class={css({ fontFamily: 'mono', fontSize: '12px', color: 'text.default' })}>
              {obj.hash.slice(0, 8)}
            </span>
            <span class={css({ marginLeft: 'auto', fontFamily: 'ui', fontSize: '10px', color: 'text.faint' })}>
              {sizeHint(obj.content)}
            </span>
          </summary>
          <pre
            class={css({
              margin: '0',
              paddingX: '12px',
              paddingY: '10px',
              borderTopWidth: '1px',
              borderTopColor: 'border.subtle',
              backgroundColor: 'surface.subtle',
              fontFamily: 'mono',
              fontSize: '11px',
              lineHeight: '[1.6]',
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-all',
              color: 'text.default',
            })}>{JSON.stringify(obj.content, null, 2)}</pre>
        </details>
      </li>
    {/each}
  </ul>
</div>
