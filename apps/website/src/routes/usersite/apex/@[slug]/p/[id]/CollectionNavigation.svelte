<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import LockIcon from '~icons/lucide/lock';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { currentSpaceSlug } from '../../current-space-slug';
  import { publicationPath } from '../../paths';
  import type { UsersiteSpacePublicationPage_CollectionNavigation_publicationView$key } from '$mearie';

  type Props = {
    publicationView$key: UsersiteSpacePublicationPage_CollectionNavigation_publicationView$key;
  };

  let { publicationView$key }: Props = $props();

  const publication = createFragment(
    graphql(`
      fragment UsersiteSpacePublicationPage_CollectionNavigation_publicationView on PublicationView {
        id

        prevInCollection {
          id
          title
          hasPassword
          passwordUnlocked

          thumbnail {
            id
            ...Img_image
          }
        }

        nextInCollection {
          id
          title
          hasPassword
          passwordUnlocked

          thumbnail {
            id
            ...Img_image
          }
        }
      }
    `),
    () => publicationView$key,
  );

  const slug = $derived(currentSpaceSlug());
  const prev = $derived(publication.data.prevInCollection);
  const next = $derived(publication.data.nextInCollection);
</script>

{#if prev || next}
  <nav
    class={flex({
      gap: '16px',
      marginTop: '40px',
      paddingTop: '24px',
      borderTopWidth: '1px',
      borderColor: 'border.hairline',
      width: 'full',
      maxWidth: 'var(--prosemirror-max-width)',
    })}
  >
    {#if prev}
      <a
        class={flex({
          flex: '1',
          gap: '12px',
          padding: '16px',
          borderRadius: '8px',
          backgroundColor: 'surface.canvas',
          cursor: 'pointer',
          transition: 'background',
          _hover: { backgroundColor: 'surface.hover' },
        })}
        href={publicationPath(slug, prev.id)}
      >
        {#if prev.thumbnail}
          <div
            class={css({
              flexShrink: '0',
              size: '48px',
              borderRadius: '6px',
              backgroundColor: 'surface.default',
              overflow: 'hidden',
            })}
          >
            <Img
              style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
              alt={prev.title}
              image$key={prev.thumbnail}
              size={48}
            />
          </div>
        {/if}

        <div class={flex({ flexDirection: 'column', justifyContent: 'center', gap: '4px', flex: '1', minWidth: '0' })}>
          <div class={flex({ alignItems: 'center', gap: '4px', color: 'text.muted', fontSize: '12px' })}>
            <Icon icon={ChevronLeftIcon} size={14} />
            <span>이전 글</span>
          </div>
          <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default', lineClamp: '2' })}>
            {#if prev.hasPassword}
              <span
                class={css({
                  display: 'inline-flex',
                  verticalAlign: 'middle',
                  marginRight: '4px',
                  color: 'text.muted',
                  transform: 'translateY(-2px)',
                })}
              >
                <Icon icon={prev.passwordUnlocked ? LockOpenIcon : LockIcon} size={12} />
              </span>
            {/if}
            {prev.title}
          </p>
        </div>
      </a>
    {:else}
      <div class={css({ flex: '1' })}></div>
    {/if}

    {#if next}
      <a
        class={flex({
          flex: '1',
          flexDirection: 'row-reverse',
          gap: '12px',
          padding: '16px',
          borderRadius: '8px',
          backgroundColor: 'surface.canvas',
          cursor: 'pointer',
          transition: 'background',
          _hover: { backgroundColor: 'surface.hover' },
        })}
        href={publicationPath(slug, next.id)}
      >
        {#if next.thumbnail}
          <div
            class={css({
              flexShrink: '0',
              size: '48px',
              borderRadius: '6px',
              backgroundColor: 'surface.default',
              overflow: 'hidden',
            })}
          >
            <Img
              style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
              alt={next.title}
              image$key={next.thumbnail}
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
          <div class={flex({ alignItems: 'center', gap: '4px', color: 'text.muted', fontSize: '12px' })}>
            <span>다음 글</span>
            <Icon icon={ChevronRightIcon} size={14} />
          </div>
          <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default', lineClamp: '2', textAlign: 'right' })}>
            {#if next.hasPassword}
              <span
                class={css({
                  display: 'inline-flex',
                  verticalAlign: 'middle',
                  marginRight: '4px',
                  color: 'text.muted',
                  transform: 'translateY(-2px)',
                })}
              >
                <Icon icon={next.passwordUnlocked ? LockOpenIcon : LockIcon} size={12} />
              </span>
            {/if}
            {next.title}
          </p>
        </div>
      </a>
    {:else}
      <div class={css({ flex: '1' })}></div>
    {/if}
  </nav>
{/if}
