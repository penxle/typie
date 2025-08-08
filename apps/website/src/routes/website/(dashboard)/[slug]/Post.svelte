<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { EntityState } from '@/enums';
  import FileXIcon from '~icons/lucide/file-x';
  import { fragment, graphql } from '$graphql';
  import { Helmet, Icon } from '$lib/components';
  import Editor from './Editor.svelte';
  import type { DashboardSlugPage_Post_query } from '$graphql';

  type Props = {
    $query: DashboardSlugPage_Post_query;
  };

  let { $query: _query }: Props = $props();

  const query = fragment(
    _query,
    graphql(`
      fragment DashboardSlugPage_Post_query on Query {
        entity(slug: $slug) {
          id
          state

          node {
            __typename

            ... on Post {
              id
            }
          }
        }

        ...Editor_query
      }
    `),
  );
</script>

{#if $query.entity.state === EntityState.ACTIVE && $query.entity.node.__typename === 'Post'}
  {#key $query.entity.node.id}
    <Editor {$query} />
  {/key}
{:else}
  <Helmet title="삭제된 포스트" />

  <div class={center({ flexDirection: 'column', gap: '20px', size: 'full', textAlign: 'center' })}>
    <Icon style={css.raw({ size: '56px', color: 'text.subtle', '& *': { strokeWidth: '[1.25px]' } })} icon={FileXIcon} />

    <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
      <h1 class={css({ fontSize: '16px', fontWeight: 'bold', color: 'text.subtle' })}>포스트가 삭제되었어요</h1>
      <p class={css({ fontSize: '14px', color: 'text.faint' })}>
        포스트가 삭제되어 더 이상 접근할 수 없어요.
        <br />
        다른 포스트를 선택해주세요
      </p>
    </div>
  </div>
{/if}
