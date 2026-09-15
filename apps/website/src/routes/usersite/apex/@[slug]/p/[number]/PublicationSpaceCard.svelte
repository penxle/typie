<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import FolderIcon from '~icons/lucide/folder';
  import LockIcon from '~icons/lucide/lock';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import { Img } from '$lib/components';
  import {
    description,
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
  import { currentSpaceSlug } from '../../current-space-slug';
  import { folderPath, publicationPath, spaceHomePath } from '../../paths';
  import type { UsersiteSpacePublicationPage_PublicationSpaceCard_publicationView$key } from '$mearie';

  type Props = {
    publicationView$key: UsersiteSpacePublicationPage_PublicationSpaceCard_publicationView$key;
  };

  let { publicationView$key }: Props = $props();

  const publication = createFragment(
    graphql(`
      fragment UsersiteSpacePublicationPage_PublicationSpaceCard_publicationView on PublicationView {
        id

        site {
          id
          name
          description
          publicationCount

          logo {
            id
            ...Img_image
          }
        }

        folder {
          id
          number
          name
          publicationCount

          thumbnail {
            id
            ...Img_image
          }

          children {
            __typename

            ... on PublicationView {
              id
            }
          }
        }

        prev {
          id
          number
          title
          hasPassword
          passwordUnlocked
        }

        next {
          id
          number
          title
          hasPassword
          passwordUnlocked
        }
      }
    `),
    () => publicationView$key,
  );

  const slug = $derived(currentSpaceSlug());
  const folder = $derived(publication.data.folder);
  const siblings = $derived(folder ? folder.children.filter((child) => child.__typename === 'PublicationView') : []);
  const position = $derived(siblings.findIndex((item) => item.id === publication.data.id) + 1);
</script>

<div
  class={css({
    borderWidth: '1px',
    borderColor: 'border.hairline',
    borderRadius: '12px',
    backgroundColor: 'surface.default',
    overflow: 'hidden',
  })}
>
  <div class={css({ padding: '20px' })}>
    <a class={css(entity)} href={spaceHomePath(slug)}>
      <Img
        style={css.raw({
          flexShrink: '0',
          size: '36px',
          borderRadius: '9px',
          objectFit: 'cover',
          boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.06)]',
        })}
        alt={`${publication.data.site.name} 로고`}
        image$key={publication.data.site.logo}
        size={96}
      />
      <span class={css(entityText)}>
        <span class={css(entityName)} data-entity-name>{publication.data.site.name}</span>
        <span class={css(entitySub)}>글 {publication.data.site.publicationCount}개</span>
      </span>
      <Icon style={css.raw({ color: 'text.hint' })} icon={ChevronRightIcon} size={14} />
    </a>

    {#if publication.data.site.description}
      <p class={css(description)}>{publication.data.site.description}</p>
    {/if}
  </div>

  {#if folder}
    <div class={css({ padding: '20px', borderTopWidth: '1px', borderColor: 'border.hairline', backgroundColor: 'surface.canvas' })}>
      <a class={css(entity)} href={folderPath(slug, folder.number)}>
        <span
          class={css({
            display: 'grid',
            placeItems: 'center',
            flexShrink: '0',
            size: '36px',
            borderRadius: '9px',
            backgroundColor: 'surface.inset',
            boxShadow: '[inset 0 0 0 1px token(colors.border.hairline)]',
            color: 'text.hint',
            overflow: 'hidden',
          })}
        >
          {#if folder.thumbnail}
            <Img
              style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
              alt={folder.name}
              image$key={folder.thumbnail}
              size={96}
            />
          {:else}
            <Icon icon={FolderIcon} size={18} />
          {/if}
        </span>
        <span class={css(entityText)}>
          <span class={css(entityName)} data-entity-name>{folder.name}</span>
          <span class={css(entitySub)}>{position} / {siblings.length}</span>
        </span>
        <Icon style={css.raw({ color: 'text.hint' })} icon={ChevronRightIcon} size={14} />
      </a>

      <div
        class={css({
          display: 'grid',
          gridTemplateColumns: { base: '1fr', sm: '1fr 1fr' },
          gap: { base: '12px', sm: '24px' },
          marginTop: '16px',
          paddingTop: '16px',
          borderTopWidth: '1px',
          borderColor: 'border.hairline',
        })}
      >
        <div class={css(side)}>
          <span class={css(sideLabel)}>
            <Icon icon={ChevronLeftIcon} size={14} />
            이전 글
          </span>
          {#if publication.data.prev}
            <a class={css(sideTitle)} href={publicationPath(slug, publication.data.prev.number)}>
              {#if publication.data.prev.hasPassword}
                <Icon
                  style={css.raw({ flexShrink: '0', color: 'text.muted' })}
                  icon={publication.data.prev.passwordUnlocked ? LockOpenIcon : LockIcon}
                  size={14}
                />
              {/if}
              <span class={css(truncate)}>{publication.data.prev.title}</span>
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
          {#if publication.data.next}
            <a class={css(sideTitle, { fontWeight: 'semibold' })} href={publicationPath(slug, publication.data.next.number)}>
              {#if publication.data.next.hasPassword}
                <Icon
                  style={css.raw({ flexShrink: '0', color: 'text.muted' })}
                  icon={publication.data.next.passwordUnlocked ? LockOpenIcon : LockIcon}
                  size={14}
                />
              {/if}
              <span class={css(truncate)}>{publication.data.next.title}</span>
            </a>
          {:else}
            <span class={css(sideEmpty)}>마지막 글이에요</span>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>
