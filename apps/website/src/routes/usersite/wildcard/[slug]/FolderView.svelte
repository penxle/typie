<script lang="ts">
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import { fragment, graphql } from '$graphql';
  import { Helmet, HorizontalDivider, Icon, Img } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex, grid } from '$styled-system/patterns';
  import ShareLinkPopover from './ShareLinkPopover.svelte';
  import type { UsersiteWildcardSlugPage_FolderView_entityView } from '$graphql';

  type Props = {
    $entityView: UsersiteWildcardSlugPage_FolderView_entityView;
  };

  let { $entityView: _entityView }: Props = $props();

  const entityView = fragment(
    _entityView,
    graphql(`
      fragment UsersiteWildcardSlugPage_FolderView_entityView on EntityView {
        id
        url

        ancestors {
          id
          url

          node {
            __typename

            ... on FolderView {
              id
              name
            }
          }
        }

        node {
          __typename

          ... on FolderView {
            id
            name
          }
        }

        children {
          id
          url

          node {
            __typename

            ... on FolderView {
              id
              name
            }

            ... on PostView {
              id
              title
              subtitle

              coverImage {
                id
                ...Img_image
              }
            }
          }
        }
      }
    `),
  );

  const folders = $derived($entityView.children.filter((child) => child.node.__typename === 'FolderView'));
  const posts = $derived($entityView.children.filter((child) => child.node.__typename === 'PostView'));
</script>

<svelte:head>
  <meta name="robots" content="noindex, nofollow" />
</svelte:head>

{#if $entityView.node.__typename === 'FolderView'}
  <Helmet
    description={`${$entityView.node.name}에서 공유된 폴더 ${folders.length}개, 포스트 ${posts.length}개를 확인하세요.`}
    title={$entityView.node.name}
  />

  <div class={flex({ flexDirection: 'column', alignItems: 'center', width: 'full', height: 'full' })}>
    <div
      class={flex({
        flexDirection: 'column',
        flexGrow: '1',
        paddingX: '20px',
        paddingTop: { base: '24px', lg: '50px' },
        paddingBottom: '80px',
        width: 'full',
        maxWidth: '860px',
        backgroundColor: 'surface.default',
      })}
    >
      <div class={flex({ alignItems: 'center', gap: '6px' })}>
        {#each $entityView.ancestors as ancestor (ancestor.id)}
          {#if ancestor.node.__typename === 'FolderView'}
            <a class={css({ fontSize: '14px', color: 'text.disabled' })} href={ancestor.url}>{ancestor.node.name}</a>
            <div class={css({ fontSize: '14px', color: 'text.disabled' })}>/</div>
          {/if}
        {/each}

        {#if $entityView.ancestors.length > 0}
          <div class={css({ fontSize: '14px' })}>{$entityView.node.name}</div>
        {/if}
      </div>

      <h1 class={css({ marginTop: '12px', fontSize: { base: '24px', lg: '28px' }, fontWeight: 'bold' })}>{$entityView.node.name}</h1>

      <div
        class={flex({
          align: 'center',
          gap: '8px',
          marginTop: { base: '12px', lg: '16px' },
          fontSize: '15px',
          fontWeight: 'medium',
          color: 'text.muted',
          lgDown: { fontSize: '14px' },
        })}
      >
        {#if folders.length > 0}
          <span class={css({ color: 'text.disabled' })}>폴더 {folders.length}개</span>
        {/if}

        {#if posts.length > 0}
          <span class={css({ color: 'text.disabled' })}>포스트 {posts.length}개</span>
        {/if}

        <ShareLinkPopover href={$entityView.url} />
      </div>

      <div class={flex({ direction: 'column', gap: '48px', marginTop: { base: '48px', lg: '60px' } })}>
        {#if folders.length > 0}
          <div>
            <p class={css({ marginBottom: '12px', fontWeight: 'semibold' })}>폴더</p>

            <div class={grid({ columns: { base: 1, lg: 2 }, gap: '10px' })}>
              {#each folders as folder (folder.id)}
                {#if folder.node.__typename === 'FolderView'}
                  <a
                    class={flex({
                      align: 'center',
                      gap: '8px',
                      borderWidth: '1px',
                      borderColor: 'border.subtle',
                      borderRadius: '8px',
                      paddingY: '12px',
                      paddingX: '16px',
                      fontSize: '15px',
                      color: 'text.subtle',
                      backgroundColor: 'surface.subtle',
                      boxShadow: 'small',
                      _hover: { backgroundColor: 'interactive.hover' },
                    })}
                    href={folder.url}
                  >
                    <Icon style={css.raw({ color: 'text.faint' })} icon={FolderIcon} />
                    <p class={css({ fontWeight: 'semibold', truncate: true })}>{folder.node.name}</p>
                  </a>
                {/if}
              {/each}
            </div>
          </div>
        {/if}

        {#if posts.length > 0}
          <div>
            <p class={css({ marginBottom: '12px', fontWeight: 'semibold' })}>포스트</p>

            <div
              class={flex({
                direction: 'column',
                gap: '2px',
                borderWidth: '1px',
                borderColor: 'border.subtle',
                borderRadius: '8px',
                padding: '2px',
                boxShadow: 'small',
              })}
            >
              {#each posts as post, i (post.id)}
                {#if i !== 0}
                  <HorizontalDivider />
                {/if}

                {#if post.node.__typename === 'PostView'}
                  <a
                    class={flex({
                      align: 'center',
                      gap: '8px',
                      borderRadius: '6px',
                      paddingX: '16px',
                      paddingY: '4px',
                      height: { base: '64px', lg: '64px' },
                      _hover: { backgroundColor: 'surface.muted' },
                    })}
                    href={post.url}
                  >
                    <Icon style={css.raw({ color: 'text.faint' })} icon={FileIcon} />

                    <div class={css({ flexGrow: '1' })}>
                      <p class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'text.subtle', lineClamp: '2' })}>
                        {post.node.title}
                      </p>
                      <p class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.muted', lineClamp: '1' })}>
                        {post.node.subtitle}
                      </p>
                    </div>

                    {#if post.node.coverImage}
                      <div
                        class={css({ borderRadius: '6px', height: 'full', aspectRatio: '[5 / 2]', backgroundColor: 'interactive.hover' })}
                      >
                        <Img
                          style={css.raw({ borderRadius: '6px' })}
                          $image={post.node.coverImage}
                          alt="커버 이미지"
                          progressive
                          ratio={5 / 2}
                          size="full"
                        />
                      </div>
                    {/if}
                  </a>
                {/if}
              {/each}
            </div>
          </div>
        {/if}

        {#if $entityView.children.length === 0}
          <p
            class={css({
              paddingX: '16px',
              paddingY: '36px',
              textAlign: 'center',
              fontSize: '14px',
              color: 'text.disabled',
            })}
          >
            포스트가 없어요
          </p>
        {/if}
      </div>
    </div>
  </div>
{/if}
