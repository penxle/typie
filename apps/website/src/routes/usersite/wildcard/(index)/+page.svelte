<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
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
              thumbnail {
                id
                ...Img_image
              }
            }

            ... on PostView {
              id
              title
              subtitle
              excerpt
              updatedAt
              thumbnail {
                id
                ...Img_image
              }
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

<div class={flex({ flexDirection: 'column', alignItems: 'center', width: 'full', minHeight: 'full' })}>
  <div
    class={flex({
      flexDirection: 'column',
      flexGrow: '1',
      paddingX: { base: '20px', md: '40px' },
      paddingTop: { base: '48px', md: '80px' },
      paddingBottom: '120px',
      width: 'full',
      maxWidth: '680px',
    })}
  >
    <header class={css({ marginBottom: { base: '56px', md: '72px' } })}>
      {#if $query.siteView.logo}
        <Img
          style={css.raw({ size: '44px', borderRadius: '10px', objectFit: 'cover', marginBottom: '20px' })}
          $image={$query.siteView.logo}
          alt={`${$query.siteView.name} 로고`}
          size={48}
        />
      {/if}

      <h1 class={css({ fontSize: '22px', fontWeight: 'bold', letterSpacing: '-0.01em', lineHeight: '[1.3]' })}>
        {$query.siteView.name}
      </h1>

      {#if folders.length > 0 || posts.length > 0}
        <p class={css({ marginTop: '8px', fontSize: '14px', color: 'text.muted' })}>
          {#if folders.length > 0}
            폴더 {folders.length}개
          {/if}
          {#if folders.length > 0 && posts.length > 0}
            ·
          {/if}
          {#if posts.length > 0}
            포스트 {posts.length}개
          {/if}
        </p>
      {/if}
    </header>

    {#if folders.length > 0}
      <section class={css({ marginBottom: '48px' })}>
        <div class={flex({ flexDirection: 'column', gap: '4px' })}>
          {#each folders as entity (entity.id)}
            {#if entity.node.__typename === 'FolderView'}
              <a
                class={flex({
                  align: 'center',
                  gap: '14px',
                  paddingY: '12px',
                  cursor: 'pointer',
                  _hover: { '& .folder-name': { color: 'text.muted' } },
                })}
                href={entity.url}
              >
                {#if entity.node.thumbnail}
                  <div
                    class={css({
                      flexShrink: '0',
                      size: '48px',
                      borderRadius: '8px',
                      backgroundColor: 'surface.subtle',
                      overflow: 'hidden',
                    })}
                  >
                    <Img
                      style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
                      $image={entity.node.thumbnail}
                      alt={entity.node.name}
                      size={48}
                    />
                  </div>
                {:else}
                  <div
                    class={flex({
                      alignItems: 'center',
                      justifyContent: 'center',
                      flexShrink: '0',
                      size: '48px',
                      borderRadius: '8px',
                      backgroundColor: 'surface.subtle',
                    })}
                  >
                    <Icon style={css.raw({ color: 'text.faint' })} icon={FolderIcon} size={18} />
                  </div>
                {/if}

                <div class={css({ flex: '1', minWidth: '0' })}>
                  <p
                    class={css({
                      fontSize: '15px',
                      fontWeight: 'medium',
                      color: 'text.default',
                      truncate: true,
                      transition: 'colors',
                    })}
                  >
                    <span class="folder-name">{entity.node.name}</span>
                  </p>
                  {#if entity.node.folderCount > 0 || entity.node.postCount > 0}
                    <p class={css({ marginTop: '2px', fontSize: '13px', color: 'text.faint' })}>
                      {#if entity.node.folderCount > 0}
                        폴더 {entity.node.folderCount}개
                      {/if}
                      {#if entity.node.folderCount > 0 && entity.node.postCount > 0}
                        ·
                      {/if}
                      {#if entity.node.postCount > 0}
                        포스트 {entity.node.postCount}개
                      {/if}
                    </p>
                  {/if}
                </div>

                <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={ChevronRightIcon} size={16} />
              </a>
            {/if}
          {/each}
        </div>
      </section>
    {/if}

    {#if posts.length > 0}
      <section>
        <div class={flex({ flexDirection: 'column' })}>
          {#each posts as entity, index (entity.id)}
            {#if entity.node.__typename === 'PostView'}
              <a
                class={flex({
                  gap: '24px',
                  paddingY: '20px',
                  borderTopWidth: index === 0 ? '0' : '1px',
                  borderColor: 'border.subtle',
                  cursor: 'pointer',
                  _hover: { '& .post-title': { color: 'text.muted' } },
                })}
                href={entity.url}
              >
                <div class={css({ flex: '1', minWidth: '0' })}>
                  <h2
                    class={css({
                      fontSize: '16px',
                      fontWeight: 'semibold',
                      lineHeight: '[1.5]',
                      letterSpacing: '-0.01em',
                      lineClamp: '2',
                      transition: 'colors',
                    })}
                  >
                    <span class="post-title">{entity.node.title}</span>
                  </h2>

                  {#if entity.node.subtitle}
                    <h3
                      class={css({
                        marginTop: '2px',
                        fontSize: '14px',
                        fontWeight: 'medium',
                        lineHeight: '[1.5]',
                        color: 'text.subtle',
                        lineClamp: '2',
                      })}
                    >
                      {entity.node.subtitle}
                    </h3>
                  {/if}

                  {#if entity.node.excerpt}
                    <p
                      class={css({
                        marginTop: '6px',
                        fontSize: '14px',
                        lineHeight: '[1.6]',
                        color: 'text.muted',
                        lineClamp: '2',
                      })}
                    >
                      {entity.node.excerpt}
                    </p>
                  {/if}

                  <p class={css({ marginTop: '10px', fontSize: '13px', color: 'text.faint' })}>
                    {dayjs(entity.node.updatedAt).format('YYYY. M. D.')}
                  </p>
                </div>

                {#if entity.node.thumbnail}
                  <div
                    class={css({
                      flexShrink: '0',
                      width: { base: '72px', md: '100px' },
                      aspectRatio: '1/1',
                      borderRadius: '6px',
                      backgroundColor: 'surface.subtle',
                      overflow: 'hidden',
                    })}
                  >
                    <Img
                      style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
                      $image={entity.node.thumbnail}
                      alt={entity.node.title}
                      size={256}
                    />
                  </div>
                {/if}
              </a>
            {/if}
          {/each}
        </div>
      </section>
    {/if}

    {#if folders.length === 0 && posts.length === 0}
      <div
        class={flex({
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          paddingY: '80px',
        })}
      >
        <p class={css({ fontSize: '14px', color: 'text.faint' })}>공유된 콘텐츠가 없어요</p>
      </div>
    {/if}
  </div>
</div>
