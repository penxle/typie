<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { titlePageColors } from '@typie/lib/title-page';
  import { css } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import LockIcon from '~icons/lucide/lock';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { currentSpaceSlug } from '../../current-space-slug';
  import { publicationPath, seriesPath, spaceHomePath } from '../../paths';
  import type { UsersiteSpacePublicationPage_PublicationSpaceCard_publicationView$key } from '$mearie';

  type Props = {
    publicationView$key: UsersiteSpacePublicationPage_PublicationSpaceCard_publicationView$key;
  };

  let { publicationView$key }: Props = $props();

  const publication = createFragment(
    graphql(`
      fragment UsersiteSpacePublicationPage_PublicationSpaceCard_publicationView on PublicationView {
        id

        space {
          id
          name
          description
          publicationCount

          logo {
            id
            ...Img_image
          }
        }

        collection {
          id
          permalink
          name
          description

          cover {
            id
            ...Img_image
          }

          publications {
            id
          }
        }

        prevInCollection {
          id
          permalink
          title
          hasPassword
          passwordUnlocked
        }

        nextInCollection {
          id
          permalink
          title
          hasPassword
          passwordUnlocked
        }
      }
    `),
    () => publicationView$key,
  );

  const slug = $derived(currentSpaceSlug());
  const collection = $derived(publication.data.collection);
  const position = $derived(collection ? collection.publications.findIndex((item) => item.id === publication.data.id) + 1 : 0);

  const entity = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '12px',
    minWidth: '0',
    _hover: { '& [data-entity-name]': { color: 'text.muted' } },
  });

  const entityText = css.raw({ flex: '1', minWidth: '0', display: 'flex', flexDirection: 'column' });
  const entityName = css.raw({ fontSize: '14px', fontWeight: 'semibold', lineHeight: '[1.4]', transition: 'colors', truncate: true });
  const entitySub = css.raw({
    fontSize: '12px',
    lineHeight: '[1.45]',
    color: 'text.hint',
    fontVariantNumeric: 'tabular-nums',
    truncate: true,
  });
  const description = css.raw({ marginTop: '12px', fontSize: '13px', lineHeight: '[1.6]', color: 'text.muted' });

  const side = css.raw({ display: 'flex', flexDirection: 'column', gap: '4px', minWidth: '0' });
  const sideLabel = css.raw({ display: 'inline-flex', alignItems: 'center', gap: '2px', fontSize: '12px', color: 'text.hint' });
  const sideTitle = css.raw({
    display: 'inline-flex',
    alignItems: 'center',
    gap: '4px',
    maxWidth: 'full',
    fontSize: '14px',
    fontWeight: 'medium',
    lineHeight: '[1.4]',
    transition: 'colors',
    _hover: { color: 'text.muted' },
  });
  const sideEmpty = css.raw({ fontSize: '14px', lineHeight: '[1.4]', color: 'text.hint' });
  const truncate = css.raw({ minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' });
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
        alt={`${publication.data.space.name} 로고`}
        image$key={publication.data.space.logo}
        size={96}
      />
      <span class={css(entityText)}>
        <span class={css(entityName)} data-entity-name>{publication.data.space.name}</span>
        <span class={css(entitySub)}>글 {publication.data.space.publicationCount}개</span>
      </span>
      <Icon style={css.raw({ color: 'text.hint' })} icon={ChevronRightIcon} size={14} />
    </a>

    {#if publication.data.space.description}
      <p class={css(description)}>{publication.data.space.description}</p>
    {/if}
  </div>

  {#if collection}
    <div class={css({ padding: '20px', borderTopWidth: '1px', borderColor: 'border.hairline', backgroundColor: 'surface.canvas' })}>
      <a class={css(entity)} href={seriesPath(slug, collection.permalink)}>
        <span
          style:background-color={collection.cover ? undefined : titlePageColors(collection.name).base}
          class={css({
            flexShrink: '0',
            width: '36px',
            height: '54px',
            borderRadius: '4px',
            backgroundColor: 'surface.inset',
            boxShadow: '[inset 0 0 0 1px token(colors.border.hairline)]',
            overflow: 'hidden',
          })}
        >
          {#if collection.cover}
            <Img
              style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
              alt={collection.name}
              image$key={collection.cover}
              size={96}
            />
          {/if}
        </span>
        <span class={css(entityText)}>
          <span class={css(entityName)} data-entity-name>{collection.name}</span>
          <span class={css(entitySub)}>{position} / {collection.publications.length}</span>
        </span>
        <Icon style={css.raw({ color: 'text.hint' })} icon={ChevronRightIcon} size={14} />
      </a>

      {#if collection.description}
        <p class={css(description, { marginTop: '8px' })}>{collection.description}</p>
      {/if}

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
          {#if publication.data.prevInCollection}
            <a class={css(sideTitle)} href={publicationPath(slug, publication.data.prevInCollection.permalink)}>
              {#if publication.data.prevInCollection.hasPassword}
                <Icon
                  style={css.raw({ flexShrink: '0', color: 'text.muted' })}
                  icon={publication.data.prevInCollection.passwordUnlocked ? LockOpenIcon : LockIcon}
                  size={14}
                />
              {/if}
              <span class={css(truncate)}>{publication.data.prevInCollection.title}</span>
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
          {#if publication.data.nextInCollection}
            <a class={css(sideTitle, { fontWeight: 'semibold' })} href={publicationPath(slug, publication.data.nextInCollection.permalink)}>
              {#if publication.data.nextInCollection.hasPassword}
                <Icon
                  style={css.raw({ flexShrink: '0', color: 'text.muted' })}
                  icon={publication.data.nextInCollection.passwordUnlocked ? LockOpenIcon : LockIcon}
                  size={14}
                />
              {/if}
              <span class={css(truncate)}>{publication.data.nextInCollection.title}</span>
            </a>
          {:else}
            <span class={css(sideEmpty)}>마지막 글이에요</span>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>
