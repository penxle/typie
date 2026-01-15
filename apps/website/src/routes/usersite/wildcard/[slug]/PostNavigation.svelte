<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import { fragment, graphql } from '$graphql';
  import { Img } from '$lib/components';
  import type { UsersiteWildcardSlugPage_PostNavigation_entityView } from '$graphql';

  type Props = {
    $entityView: UsersiteWildcardSlugPage_PostNavigation_entityView;
  };

  let { $entityView: _entityView }: Props = $props();

  const entityView = fragment(
    _entityView,
    graphql(`
      fragment UsersiteWildcardSlugPage_PostNavigation_entityView on EntityView {
        id

        prevPost {
          id
          url

          node {
            __typename

            ... on PostView {
              id
              title
              thumbnail {
                id
                ...Img_image
              }
            }
          }
        }

        nextPost {
          id
          url

          node {
            __typename

            ... on PostView {
              id
              title
              thumbnail {
                id
                ...Img_image
              }
            }
          }
        }
      }
    `),
  );

  const hasPrev = $derived($entityView.prevPost !== null);
  const hasNext = $derived($entityView.nextPost !== null);
</script>

{#if hasPrev || hasNext}
  <nav
    class={flex({
      gap: '16px',
      marginTop: '40px',
      paddingTop: '24px',
      borderTopWidth: '1px',
      borderColor: 'border.subtle',
      width: 'full',
      maxWidth: 'var(--prosemirror-max-width)',
    })}
  >
    {#if hasPrev && $entityView.prevPost?.node.__typename === 'PostView'}
      <a
        class={flex({
          flex: '1',
          gap: '12px',
          padding: '16px',
          borderRadius: '8px',
          backgroundColor: 'surface.subtle',
          cursor: 'pointer',
          transition: 'background',
          _hover: { backgroundColor: 'surface.muted' },
        })}
        href={$entityView.prevPost.url}
      >
        {#if $entityView.prevPost.node.thumbnail}
          <div
            class={css({
              flexShrink: '0',
              size: '48px',
              borderRadius: '6px',
              backgroundColor: 'surface.muted',
              overflow: 'hidden',
            })}
          >
            <Img
              style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
              $image={$entityView.prevPost.node.thumbnail}
              alt={$entityView.prevPost.node.title}
              size={48}
            />
          </div>
        {/if}

        <div class={flex({ flexDirection: 'column', justifyContent: 'center', gap: '4px', flex: '1', minWidth: '0' })}>
          <div class={flex({ alignItems: 'center', gap: '4px', color: 'text.faint', fontSize: '12px' })}>
            <Icon icon={ChevronLeftIcon} size={14} />
            <span>이전 글</span>
          </div>
          <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default', lineClamp: '2' })}>
            {$entityView.prevPost.node.title}
          </p>
        </div>
      </a>
    {:else}
      <div class={css({ flex: '1' })}></div>
    {/if}

    {#if hasNext && $entityView.nextPost?.node.__typename === 'PostView'}
      <a
        class={flex({
          flex: '1',
          flexDirection: 'row-reverse',
          gap: '12px',
          padding: '16px',
          borderRadius: '8px',
          backgroundColor: 'surface.subtle',
          cursor: 'pointer',
          transition: 'background',
          _hover: { backgroundColor: 'surface.muted' },
        })}
        href={$entityView.nextPost.url}
      >
        {#if $entityView.nextPost.node.thumbnail}
          <div
            class={css({
              flexShrink: '0',
              size: '48px',
              borderRadius: '6px',
              backgroundColor: 'surface.muted',
              overflow: 'hidden',
            })}
          >
            <Img
              style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
              $image={$entityView.nextPost.node.thumbnail}
              alt={$entityView.nextPost.node.title}
              size={48}
            />
          </div>
        {/if}

        <div
          class={flex({
            flexDirection: 'column',
            alignItems: 'flex-end',
            justifyContent: 'center',
            gap: '4px',
            flex: '1',
            minWidth: '0',
          })}
        >
          <div class={flex({ alignItems: 'center', gap: '4px', color: 'text.faint', fontSize: '12px' })}>
            <span>다음 글</span>
            <Icon icon={ChevronRightIcon} size={14} />
          </div>
          <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default', lineClamp: '2', textAlign: 'right' })}>
            {$entityView.nextPost.node.title}
          </p>
        </div>
      </a>
    {:else}
      <div class={css({ flex: '1' })}></div>
    {/if}
  </nav>
{/if}
