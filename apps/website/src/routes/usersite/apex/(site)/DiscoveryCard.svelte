<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import dayjs from 'dayjs';
  import { Img } from '$lib/components';
  import PublicationCard from '$lib/usersite/PublicationCard.svelte';
  import { graphql } from '$mearie';
  import { folderPath, tagPath } from '../@[slug]/paths';
  import { pickSpaceDate } from '../@[slug]/space-date';
  import { discoveryTagPath } from './paths';
  import type { UsersiteApex_DiscoveryCard_publicationView$key } from '$mearie';

  type Props = {
    publicationView$key: UsersiteApex_DiscoveryCard_publicationView$key;
    enter?: boolean;
    context?: 'discovery' | 'space';
    showFolder?: boolean;
    titleHtml?: string | null;
    excerptHtml?: string | null;
  };

  let {
    publicationView$key,
    enter = false,
    context = 'discovery',
    showFolder = true,
    titleHtml = null,
    excerptHtml = null,
  }: Props = $props();

  const publication = createFragment(
    graphql(`
      fragment UsersiteApex_DiscoveryCard_publicationView on PublicationView {
        id
        number
        title
        subtitle
        excerpt
        tags
        publishedAt
        updatedAt
        reactionCount
        hasPassword
        passwordUnlocked
        url

        ancestors {
          id
          number
          name
        }

        thumbnail {
          id
          ...Img_image
        }

        site {
          id
          name
          slug
          url
          dateDisplay

          logo {
            id
            ...Img_image
          }
        }
      }
    `),
    () => publicationView$key,
  );

  const date = $derived(
    context === 'space' ? pickSpaceDate(publication.data.site.dateDisplay, publication.data) : publication.data.publishedAt,
  );
  const timestamp = $derived(date === null ? null : dayjs(date).valueOf());
  const folders = $derived(
    context === 'space' && showFolder
      ? publication.data.ancestors.map((ancestor) => ({
          name: ancestor.name,
          href: folderPath(publication.data.site.slug, ancestor.number),
        }))
      : [],
  );
  const tagHref = (tag: string) => (context === 'space' ? tagPath(publication.data.site.slug, tag) : discoveryTagPath(tag));
</script>

{#snippet siteLead()}
  <a
    class={flex({
      alignItems: 'center',
      gap: '6px',
      minWidth: '0',
      fontWeight: 'medium',
      color: 'text.muted',
      _hover: { '& span': { color: 'text.default' } },
    })}
    href={publication.data.site.url}
  >
    <Img
      style={css.raw({
        flexShrink: '0',
        size: '20px',
        borderRadius: '5px',
        objectFit: 'cover',
        boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.06)]',
      })}
      alt={`${publication.data.site.name} 로고`}
      image$key={publication.data.site.logo}
      size={48}
    />
    <span class={css({ overflow: 'hidden', textOverflow: 'ellipsis', transition: 'colors' })}>{publication.data.site.name}</span>
  </a>
{/snippet}

{#snippet thumbnail()}
  {#if publication.data.thumbnail}
    <Img
      style={css.raw({
        width: 'full',
        height: 'full',
        objectFit: 'cover',
        transition: '[transform 240ms cubic-bezier(0.32, 0.72, 0, 1)]',
        _motionReduce: { transition: '[none]' },
      })}
      alt={publication.data.title}
      data-card-cover
      image$key={publication.data.thumbnail}
      size={256}
    />
  {/if}
{/snippet}

<PublicationCard
  {enter}
  excerpt={publication.data.excerpt}
  {excerptHtml}
  {folders}
  hasPassword={publication.data.hasPassword}
  href={publication.data.url}
  lead={context === 'discovery' ? siteLead : undefined}
  passwordUnlocked={publication.data.passwordUnlocked}
  reactionCount={publication.data.reactionCount}
  subtitle={publication.data.subtitle}
  {tagHref}
  tags={publication.data.tags}
  thumbnail={publication.data.thumbnail ? thumbnail : undefined}
  {timestamp}
  title={publication.data.title}
  {titleHtml}
/>
