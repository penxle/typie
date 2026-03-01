<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import type { UsersiteWildcardSlugPage_ContentNavigation_entityView$key } from '$mearie';

  type Props = {
    entityView$key: UsersiteWildcardSlugPage_ContentNavigation_entityView$key;
  };

  let { entityView$key }: Props = $props();

  const entityView = createFragment(
    graphql(`
      fragment UsersiteWildcardSlugPage_ContentNavigation_entityView on EntityView {
        id

        prev {
          id
          slug

          node {
            __typename

            ... on DocumentView {
              id
              title
              thumbnail {
                id
                ...Img_image
              }
            }
          }
        }

        next {
          id
          slug

          node {
            __typename

            ... on DocumentView {
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
    () => entityView$key,
  );

  const prevNode = $derived(entityView.data.prev?.node.__typename === 'DocumentView' ? entityView.data.prev.node : null);
  const nextNode = $derived(entityView.data.next?.node.__typename === 'DocumentView' ? entityView.data.next.node : null);
</script>

{#if prevNode || nextNode}
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
    {#if prevNode && entityView.data.prev}
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
        href={`/${entityView.data.prev.slug}`}
      >
        {#if prevNode.thumbnail}
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
              alt={prevNode.title}
              image$key={prevNode.thumbnail}
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
            {prevNode.title}
          </p>
        </div>
      </a>
    {:else}
      <div class={css({ flex: '1' })}></div>
    {/if}

    {#if nextNode && entityView.data.next}
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
        href={`/${entityView.data.next.slug}`}
      >
        {#if nextNode.thumbnail}
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
              alt={nextNode.title}
              image$key={nextNode.thumbnail}
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
            {nextNode.title}
          </p>
        </div>
      </a>
    {:else}
      <div class={css({ flex: '1' })}></div>
    {/if}
  </nav>
{/if}
