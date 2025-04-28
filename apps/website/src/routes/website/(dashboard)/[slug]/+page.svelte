<script lang="ts">
  import { EntityState } from '@/enums';
  import FileXIcon from '~icons/lucide/file-x';
  import { graphql } from '$graphql';
  import { Helmet, Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import Editor from './Editor.svelte';

  const query = graphql(`
    query DashboardSlugPage_Query($slug: String!) {
      post(slug: $slug) {
        id

        entity {
          id
          state
        }
      }

      ...Editor_query
    }
  `);
</script>

{#if $query.post.entity.state === EntityState.ACTIVE}
  {#key $query.post.id}
    <Editor {$query} />
  {/key}
{:else}
  <Helmet title="삭제된 포스트" />

  <div class={center({ flexDirection: 'column', gap: '20px', size: 'full', textAlign: 'center' })}>
    <Icon style={css.raw({ size: '56px', color: 'gray.700', '& *': { strokeWidth: '[1.25px]' } })} icon={FileXIcon} />

    <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
      <h1 class={css({ fontSize: '16px', fontWeight: 'bold', color: 'gray.700' })}>포스트가 삭제되었어요</h1>
      <p class={css({ fontSize: '14px', color: 'gray.500' })}>
        포스트가 삭제되어 더 이상 접근할 수 없어요.
        <br />
        다른 포스트를 선택해주세요
      </p>
    </div>
  </div>
{/if}
