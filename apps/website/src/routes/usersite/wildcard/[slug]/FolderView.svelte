<script lang="ts">
  import EllipsisVerticalIcon from '~icons/lucide/ellipsis-vertical';
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

{#if $entityView.node.__typename === 'FolderView'}
  <Helmet
    description={`${$entityView.node.name}에서 공유된 폴더 ${folders.length}개, 포스트 ${posts.length}개를 확인하세요.`}
    image={{ size: 'large', src: 'https://typie.net/opengraph/default.png' }}
    title={$entityView.node.name}
  />

  <div class={flex({ flexDirection: 'column', alignItems: 'center', width: 'full', minHeight: 'screen' })}>
    <div
      class={flex({
        flexDirection: 'column',
        flexGrow: '1',
        paddingX: '20px',
        paddingTop: '50px',
        paddingBottom: '80px',
        width: 'full',
        maxWidth: '860px',
        backgroundColor: 'white',
      })}
    >
      <div class={flex({ alignItems: 'center', gap: '6px' })}>
        {#each $entityView.ancestors as ancestor (ancestor.id)}
          {#if ancestor.node.__typename === 'FolderView'}
            <a class={css({ fontSize: '14px', color: 'gray.400' })} href={ancestor.url}>{ancestor.node.name}</a>
            <div class={css({ fontSize: '14px', color: 'gray.300' })}>/</div>
          {/if}
        {/each}

        {#if $entityView.ancestors.length > 0}
          <div class={css({ fontSize: '14px' })}>{$entityView.node.name}</div>
        {/if}
      </div>

      <h1 class={css({ marginTop: '12px', fontSize: '28px', fontWeight: 'bold' })}>{$entityView.node.name}</h1>

      <div class={flex({ align: 'center', justify: 'space-between', gap: '24px', marginTop: '20px' })}>
        <div class={css({ fontWeight: 'medium', color: 'gray.400' })}>
          {#if folders.length > 0}
            <span>폴더 {folders.length}개</span>
          {/if}

          {#if posts.length > 0}
            <span>포스트 {posts.length}개</span>
          {/if}
        </div>

        <div class={flex({ align: 'center', marginLeft: 'auto', gap: '16px', color: 'gray.600' })}>
          <ShareLinkPopover href={$entityView.url} />

          <button type="button">
            <Icon icon={EllipsisVerticalIcon} size={18} />
          </button>
        </div>
      </div>

      <div class={flex({ direction: 'column', gap: '20px', marginTop: '60px' })}>
        <div class={grid({ columns: 3, gap: '10px' })}>
          {#each folders as folder (folder.id)}
            {#if folder.node.__typename === 'FolderView'}
              <a
                class={flex({
                  align: 'center',
                  gap: '8px',
                  borderWidth: '1px',
                  borderColor: 'gray.100',
                  borderRadius: '12px',
                  padding: '16px',
                  backgroundColor: 'gray.50',
                  boxShadow: 'small',
                  _hover: { backgroundColor: 'gray.200' },
                })}
                href={folder.url}
              >
                <Icon style={css.raw({ color: 'gray.500' })} icon={FolderIcon} />
                <p class={css({ fontWeight: 'semibold', truncate: true })}>{folder.node.name}</p>
              </a>
            {/if}
          {/each}
        </div>

        <div class={css({ borderWidth: '1px', borderColor: 'gray.100', borderRadius: '12px', boxShadow: 'small' })}>
          <div class={flex({ align: 'center', gap: '8px', borderBottomWidth: '1px', borderBottomColor: 'gray.100', padding: '16px' })}>
            <p class={css({ fontWeight: 'semibold', truncate: true })}>{$entityView.node.name}</p>

            <p class={css({ flex: 'none', fontSize: '12px', color: 'gray.400' })}>{posts.length}개의 포스트</p>
          </div>

          {#each posts as post, i (post.id)}
            {#if i !== 0}
              <HorizontalDivider />
            {/if}

            {#if post.node.__typename === 'PostView'}
              <a
                class={flex({
                  align: 'center',
                  gap: '24px',
                  paddingX: '16px',
                  paddingY: '8px',
                  height: '96px',
                  _last: { borderBottomRadius: '12px' },
                  _hover: { backgroundColor: 'gray.100' },
                })}
                href={post.url}
              >
                <div class={css({ flexGrow: '1' })}>
                  <p class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'gray.700', lineClamp: '2' })}>{post.node.title}</p>
                  <p class={css({ fontSize: '13px', fontWeight: 'medium', color: 'gray.600', lineClamp: '1' })}>{post.node.subtitle}</p>
                </div>

                {#if post.node.coverImage}
                  <div class={css({ borderRadius: '6px', height: 'full', aspectRatio: '[5 / 2]', backgroundColor: 'gray.300' })}>
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
          {:else}
            <p class={css({ paddingX: '16px', paddingY: '36px', textAlign: 'center', fontSize: '14px', color: 'gray.400' })}>
              포스트가 없어요
            </p>
          {/each}
        </div>
      </div>
    </div>
  </div>
{/if}
