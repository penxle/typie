<script lang="ts">
  import { EntityState } from '@/enums';
  import FileXIcon from '~icons/lucide/file-x';
  import { afterNavigate } from '$app/navigation';
  import { graphql } from '$graphql';
  import { Helmet, Icon } from '$lib/components';
  import { LocalStore } from '$lib/state';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import Editor from './Editor.svelte';

  const query = graphql(`
    query DashboardSlugPage_Query($slug: String!) {
      me @required {
        id
      }

      post(slug: $slug) {
        id

        entity {
          id
          slug
          state

          site {
            id
          }

          user {
            id
          }
        }
      }

      ...Editor_query
    }
  `);

  afterNavigate(() => {
    if ($query.me.id === $query.post.entity.user.id) {
      const lvp = LocalStore.get<Record<string, string>>('typie:lvp') ?? {};
      lvp[$query.post.entity.site.id] = $query.post.entity.slug;
      LocalStore.set('typie:lvp', lvp);
    }
  });
</script>

{#if $query.post.entity.state === EntityState.ACTIVE}
  {#key $query.post.id}
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
