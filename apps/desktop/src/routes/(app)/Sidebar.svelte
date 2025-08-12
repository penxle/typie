<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { fragment, graphql } from '$graphql';
  import Editor from './@pages/editor/Page.svelte';
  import { tabState } from './tabs.svelte';
  import type { Sidebar_user } from '$graphql';

  type Props = {
    $user: Sidebar_user;
  };

  const { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment Sidebar_user on User {
        id

        sites {
          id

          entities {
            id
            slug

            node {
              __typename

              ... on Post {
                id
                title
              }
            }
          }
        }
      }
    `),
  );

  const posts = $derived($user.sites[0].entities.filter((entity) => entity.node.__typename === 'Post'));
</script>

<aside
  class={flex({
    flexDirection: 'column',
    flexShrink: '0',
    width: '200px',
    height: 'full',
    backgroundColor: 'surface.subtle',
  })}
>
  <div style:-webkit-app-region="drag" class={css({ flexShrink: '0', height: '40px' })} data-tauri-drag-region></div>

  <div class={flex({ flexGrow: '1', flexDirection: 'column', gap: '4px', overflowY: 'auto' })}>
    {#each posts as entity (entity.id)}
      {#if entity.node.__typename === 'Post'}
        <button
          class={css({ paddingX: '8px', paddingY: '4px', textAlign: 'left' })}
          onclick={() => {
            tabState.navigate(tabState.active.id, Editor, { slug: entity.slug });
          }}
          type="button"
        >
          <div class={css({ fontSize: '15px', lineClamp: '1' })}>{entity.node.title}</div>
        </button>
      {/if}
    {/each}
  </div>
</aside>
