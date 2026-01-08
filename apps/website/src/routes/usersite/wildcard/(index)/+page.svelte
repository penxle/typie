<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import { graphql } from '$graphql';
  import { Img } from '$lib/components';

  const query = graphql(`
    query UsersiteWildcardIndexPage_Query($origin: String!) {
      siteView(origin: $origin) {
        id
        name

        logo {
          id
          ...Img_image
        }

        entities {
          id
          url

          node {
            __typename

            ... on FolderView {
              id
              name
              folderCount
              postCount
            }

            ... on PostView {
              id
              title
              subtitle
              excerpt
            }
          }
        }
      }
    }
  `);

  const folders = $derived($query.siteView.entities.filter((entity) => entity.node.__typename === 'FolderView'));
  const posts = $derived($query.siteView.entities.filter((entity) => entity.node.__typename === 'PostView'));
</script>

<svelte:head>
  <meta name="robots" content="noindex, nofollow" />
</svelte:head>

<Helmet
  description={`${$query.siteView.name}에서 공유된 폴더 ${folders.length}개, 포스트 ${posts.length}개를 확인하세요.`}
  title={$query.siteView.name}
/>

<div class={flex({ flexDirection: 'column', alignItems: 'center', width: 'full', height: 'full' })}>
  <div
    class={flex({
      flexDirection: 'column',
      flexGrow: '1',
      paddingX: '20px',
      paddingTop: { base: '24px', md: '48px' },
      paddingBottom: '80px',
      width: 'full',
      maxWidth: '860px',
      backgroundColor: 'surface.default',
    })}
  >
    <div class={flex({ alignItems: 'center', gap: '12px', marginBottom: '8px' })}>
      {#if $query.siteView.logo}
        <Img
          style={css.raw({ size: '48px', borderRadius: '8px', objectFit: 'cover' })}
          $image={$query.siteView.logo}
          alt={`${$query.siteView.name} 로고`}
          size={48}
        />
      {/if}
    </div>

    <h1 class={css({ fontSize: { base: '24px', md: '28px' }, fontWeight: 'bold' })}>{$query.siteView.name}</h1>

    <div
      class={flex({
        align: 'center',
        gap: '8px',
        marginTop: { base: '8px', md: '4px' },
        fontSize: '14px',
        fontWeight: 'medium',
        color: 'text.faint',
        mdDown: { fontSize: '14px' },
      })}
    >
      {#if folders.length > 0}
        <span>폴더 {folders.length}개</span>
      {/if}

      {#if posts.length > 0}
        <span>포스트 {posts.length}개</span>
      {/if}
    </div>

    <div class={flex({ direction: 'column', gap: '32px', marginTop: { base: '48px', md: '60px' } })}>
      <div
        class={flex({
          direction: 'column',
          gap: '4px',
          padding: '2px',
        })}
      >
        {#each $query.siteView.entities as entity (entity.id)}
          {#if entity.node.__typename === 'PostView'}
            <a
              class={flex({
                align: 'center',
                gap: '12px',
                borderWidth: '1px',
                borderColor: 'border.subtle',
                borderRadius: '4px',
                paddingX: { base: '12px', md: '16px' },
                backgroundColor: 'surface.default',
                height: '62px',
                _hover: { backgroundColor: 'surface.subtle' },
              })}
              href={entity.url}
            >
              <Icon style={css.raw({ color: 'text.faint' })} icon={FileIcon} size={14} />

              <div class={css({ flexGrow: '1', truncate: true })}>
                <p class={css({ fontSize: '14px', fontWeight: 'semibold', truncate: true })}>
                  {entity.node.title}
                </p>

                <div class={flex({ align: 'center', gap: '4px', fontSize: '13px' })}>
                  {#if entity.node.subtitle}
                    <p class={css({ fontWeight: 'medium', color: 'text.muted' })}>
                      {entity.node.subtitle}
                    </p>

                    <p>|</p>
                  {/if}
                  <p class={css({ color: 'text.faint', truncate: true })}>
                    {entity.node.excerpt || '(내용 없음)'}
                  </p>
                </div>
              </div>
            </a>
          {:else if entity.node.__typename === 'FolderView'}
            <a
              class={flex({
                align: 'center',
                gap: '12px',
                borderWidth: '1px',
                borderColor: 'border.subtle',
                borderRadius: '4px',
                paddingX: { base: '12px', md: '16px' },
                fontSize: '14px',
                color: 'text.subtle',
                height: '62px',
                backgroundColor: 'surface.subtle',
                truncate: true,
                _hover: { backgroundColor: 'surface.muted' },
              })}
              href={entity.url}
            >
              <Icon style={css.raw({ color: 'text.faint' })} icon={FolderIcon} size={14} />
              <div class={css({ truncate: true })}>
                <p class={css({ fontWeight: 'semibold', truncate: true })}>{entity.node.name}</p>

                <p class={css({ color: 'text.muted', fontWeight: 'medium', fontSize: '13px' })}>
                  {#if entity.node.folderCount > 0}
                    폴더 {entity.node.folderCount}개
                  {/if}
                  {#if entity.node.folderCount > 0 && entity.node.postCount}
                    ·
                  {/if}
                  {#if entity.node.postCount > 0}
                    포스트 {entity.node.postCount}개
                  {/if}
                </p>
              </div>

              <Icon style={css.raw({ marginLeft: 'auto', color: 'text.faint' })} icon={ChevronRightIcon} />
            </a>
          {/if}
        {:else}
          <p
            class={css({
              paddingX: { base: '12px', md: '16px' },
              paddingY: '36px',
              textAlign: 'center',
              fontSize: '14px',
              color: 'text.disabled',
            })}
          >
            공유된 콘텐츠가 없어요
          </p>
        {/each}
      </div>
    </div>
  </div>
</div>
