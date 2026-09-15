<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import FolderIcon from '~icons/lucide/folder';
  import LockIcon from '~icons/lucide/lock';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import {
    entity,
    entityName,
    entitySub,
    entityText,
    side,
    sideEmpty,
    sideLabel,
    sideTitle,
    truncate,
  } from '$lib/usersite/reading-context-card';
  import { graphql } from '$mearie';
  import { folderCardBlocks } from './folder-card';
  import type { UsersiteApexSlugPage_FolderCard_entityView$key } from '$mearie';

  type Props = {
    entityView$key: UsersiteApexSlugPage_FolderCard_entityView$key;
  };

  let { entityView$key }: Props = $props();

  const entityView = createFragment(
    graphql(`
      fragment UsersiteApexSlugPage_FolderCard_entityView on EntityView {
        id

        ancestors {
          id
          slug

          node {
            __typename

            ... on FolderView {
              id
              name
              folderCount
              documentCount
            }
          }
        }

        prev {
          id
          slug

          node {
            __typename

            ... on DocumentView {
              id
              title
              hasPassword
              passwordUnlocked
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
              hasPassword
              passwordUnlocked
            }
          }
        }
      }
    `),
    () => entityView$key,
  );

  const folder = $derived.by(() => {
    const ancestor = entityView.data.ancestors.findLast((item) => item.node.__typename === 'FolderView');
    return ancestor && ancestor.node.__typename === 'FolderView' ? { slug: ancestor.slug, ...ancestor.node } : null;
  });
  const prev = $derived(
    entityView.data.prev && entityView.data.prev.node.__typename === 'DocumentView'
      ? { slug: entityView.data.prev.slug, ...entityView.data.prev.node }
      : null,
  );
  const next = $derived(
    entityView.data.next && entityView.data.next.node.__typename === 'DocumentView'
      ? { slug: entityView.data.next.slug, ...entityView.data.next.node }
      : null,
  );
  const blocks = $derived(folderCardBlocks({ folder: folder !== null, prev: prev !== null, next: next !== null }));
</script>

{#if blocks.folder || blocks.siblings}
  <div
    class={css({
      borderWidth: '1px',
      borderColor: 'border.hairline',
      borderRadius: '12px',
      backgroundColor: 'surface.default',
      overflow: 'hidden',
    })}
  >
    {#if folder}
      <div class={css({ padding: '20px' })}>
        <a class={css(entity)} href={`/s/${folder.slug}`}>
          <span
            class={css({
              display: 'grid',
              placeItems: 'center',
              flexShrink: '0',
              size: '36px',
              borderRadius: '9px',
              backgroundColor: 'surface.canvas',
              color: 'text.hint',
            })}
          >
            <Icon icon={FolderIcon} size={18} />
          </span>
          <span class={css(entityText)}>
            <span class={css(entityName)} data-entity-name>{folder.name}</span>
            {#if folder.folderCount > 0 || folder.documentCount > 0}
              <span class={css(entitySub)}>
                {#if folder.folderCount > 0}
                  폴더 {folder.folderCount}개
                {/if}
                {#if folder.folderCount > 0 && folder.documentCount > 0}
                  ·
                {/if}
                {#if folder.documentCount > 0}
                  문서 {folder.documentCount}개
                {/if}
              </span>
            {/if}
          </span>
          <Icon style={css.raw({ color: 'text.hint' })} icon={ChevronRightIcon} size={14} />
        </a>
      </div>
    {/if}

    {#if blocks.siblings}
      <div
        class={css(
          { padding: '20px', backgroundColor: 'surface.canvas' },
          blocks.folder && { borderTopWidth: '1px', borderColor: 'border.hairline' },
        )}
      >
        <div
          class={css({
            display: 'grid',
            gridTemplateColumns: { base: '1fr', sm: '1fr 1fr' },
            gap: { base: '12px', sm: '24px' },
          })}
        >
          <div class={css(side)}>
            <span class={css(sideLabel)}>
              <Icon icon={ChevronLeftIcon} size={14} />
              이전 글
            </span>
            {#if prev}
              <a class={css(sideTitle)} href={`/s/${prev.slug}`}>
                {#if prev.hasPassword}
                  <Icon
                    style={css.raw({ flexShrink: '0', color: 'text.muted' })}
                    icon={prev.passwordUnlocked ? LockOpenIcon : LockIcon}
                    size={14}
                  />
                {/if}
                <span class={css(truncate)}>{prev.title}</span>
              </a>
            {:else}
              <span class={css(sideEmpty)}>첫 글이에요</span>
            {/if}
          </div>

          <div class={css(side, { sm: { alignItems: 'flex-end', textAlign: 'right' } })}>
            <span class={css(sideLabel)}>
              다음 글
              <Icon icon={ChevronRightIcon} size={14} />
            </span>
            {#if next}
              <a class={css(sideTitle, { fontWeight: 'semibold' })} href={`/s/${next.slug}`}>
                {#if next.hasPassword}
                  <Icon
                    style={css.raw({ flexShrink: '0', color: 'text.muted' })}
                    icon={next.passwordUnlocked ? LockOpenIcon : LockIcon}
                    size={14}
                  />
                {/if}
                <span class={css(truncate)}>{next.title}</span>
              </a>
            {:else}
              <span class={css(sideEmpty)}>마지막 글이에요</span>
            {/if}
          </div>
        </div>
      </div>
    {/if}
  </div>
{/if}
