<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { EntityState } from '@/enums';
  import FileXIcon from '~icons/lucide/file-x';
  import { graphql } from '$graphql';
  import { tabState } from '../../tabs.svelte';
  import Canvas from './@canvas/Canvas.svelte';
  import Editor from './Editor.svelte';

  type Props = {
    tabId: string;
    slug: string;
  };

  const { tabId, slug }: Props = $props();

  const query = graphql(`
    query DashboardSlugPage_Query($slug: String!) @client {
      me @required {
        id
      }

      entity(slug: $slug) {
        id
        slug
        state

        site {
          id
        }

        user {
          id
        }

        node {
          __typename

          ... on Post {
            id
            title
          }

          ... on Canvas {
            id
            title
          }
        }
      }

      ...Canvas_query
      ...Editor_query
    }
  `);

  const viewEntity = graphql(`
    mutation DashboardSlugPage_ViewEntity_Mutation($input: ViewEntityInput!) {
      viewEntity(input: $input) {
        id

        user {
          id

          recentlyViewedEntities {
            id
          }
        }
      }
    }
  `);

  let isViewed = $state(false);

  $effect(() => {
    query.load({ slug });
  });

  $effect(() => {
    if ($query) {
      tabState.setTitle(
        tabId,
        $query.entity.node.__typename === 'Post'
          ? $query.entity.node.title
          : $query.entity.node.__typename === 'Canvas'
            ? $query.entity.node.title
            : '',
      );
    }
  });

  $effect(() => {
    if (!$query || isViewed) {
      return;
    }

    if ($query.me.id === $query.entity.user.id && $query.entity.state === EntityState.ACTIVE) {
      viewEntity({ entityId: $query.entity.id });
      isViewed = true;
    }
  });
</script>

{#if $query}
  {#if $query.entity.state === EntityState.ACTIVE}
    {#key $query.entity.id}
      {#if $query.entity.node.__typename === 'Post'}
        <Editor {$query} />
      {:else if $query.entity.node.__typename === 'Canvas'}
        <Canvas {$query} />
      {/if}
    {/key}
  {:else}
    {@const name = $query.entity.node.__typename === 'Post' ? '포스트' : '캔버스'}
    <div class={center({ flexDirection: 'column', gap: '20px', size: 'full', textAlign: 'center' })}>
      <Icon style={css.raw({ size: '56px', color: 'text.subtle', '& *': { strokeWidth: '[1.25px]' } })} icon={FileXIcon} />

      <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
        <h1 class={css({ fontSize: '16px', fontWeight: 'bold', color: 'text.subtle' })}>{name}가 삭제되었어요</h1>
        <p class={css({ fontSize: '14px', color: 'text.faint' })}>
          {name}가 삭제되어 더 이상 접근할 수 없어요.
          <br />
          다른 {name}를 선택해주세요
        </p>
      </div>
    </div>
  {/if}
{/if}
