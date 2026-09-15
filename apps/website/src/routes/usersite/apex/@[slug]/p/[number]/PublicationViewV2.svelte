<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import dayjs from 'dayjs';
  import { env } from '$env/dynamic/public';
  import { Img } from '$lib/components';
  import { parseDocumentViewLayoutMode } from '$lib/usersite/document-view-layout';
  import ReadingActionMenu from '$lib/usersite/ReadingActionMenu.svelte';
  import ReadingKicker from '$lib/usersite/ReadingKicker.svelte';
  import ReadingReactions from '$lib/usersite/ReadingReactions.svelte';
  import ReadingTitle from '$lib/usersite/ReadingTitle.svelte';
  import ReadingView from '$lib/usersite/ReadingView.svelte';
  import ShareLinkPopover from '$lib/usersite/ShareLinkPopover.svelte';
  import { graphql } from '$mearie';
  import { getUsersiteChrome } from '../../../../chrome.svelte';
  import { currentSpaceSlug } from '../../current-space-slug';
  import { folderPath, spaceHomePath, tagPath } from '../../paths';
  import { pickSpaceDate } from '../../space-date';
  import TagChip from '../../TagChip.svelte';
  import PublicationRecent from './PublicationRecent.svelte';
  import PublicationSpaceCard from './PublicationSpaceCard.svelte';
  import type {
    UsersiteSpacePublicationPage_PublicationViewV2_publicationView$key,
    UsersiteSpacePublicationPage_PublicationViewV2_user$key,
  } from '$mearie';

  type Props = {
    publicationView$key: UsersiteSpacePublicationPage_PublicationViewV2_publicationView$key;
    user$key: UsersiteSpacePublicationPage_PublicationViewV2_user$key | null | undefined;
  };

  let { publicationView$key, user$key }: Props = $props();

  const chrome = getUsersiteChrome();
  const slug = $derived(currentSpaceSlug());
  let titleEl = $state<HTMLElement>();

  const publication = createFragment(
    graphql(`
      fragment UsersiteSpacePublicationPage_PublicationViewV2_publicationView on PublicationView {
        id
        number
        title
        subtitle
        hasPassword
        publishedAt
        updatedAt
        tags
        url
        layoutMode

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
          dateDisplay

          logo {
            id
            ...Img_image
          }
        }

        ...UsersiteReadingView_document
        ...UsersiteReadingReactions_document
        ...UsersiteReadingActionMenu_document
        ...UsersiteSpacePublicationPage_PublicationSpaceCard_publicationView
        ...UsersiteSpacePublicationPage_PublicationRecent_publicationView
      }
    `),
    () => publicationView$key,
  );

  const user = createFragment(
    graphql(`
      fragment UsersiteSpacePublicationPage_PublicationViewV2_user on User {
        id
        ...UsersiteReadingView_user
      }
    `),
    () => user$key,
  );

  const [unlockPublicationView] = createMutation(
    graphql(`
      mutation UsersiteSpacePublicationPage_PublicationViewV2_UnlockPublicationView_Mutation($input: UnlockPublicationViewInput!) {
        unlockPublicationView(input: $input) {
          id

          documentBody: body {
            __typename

            ... on DocumentViewBodyAvailableV2 {
              graph
            }

            ... on DocumentViewBodyUnavailable {
              reason
            }
          }

          assets {
            __typename

            ... on Image {
              id
              url
              originalUrl
              width
              height
              placeholder
            }

            ... on File {
              id
              url
              name
              size
            }

            ... on Embed {
              id
              url
              title
              description
              thumbnailUrl
              html
            }

            ... on DocumentArchivedNode {
              id
              content
            }
          }
        }
      }
    `),
  );

  const document = $derived(publication.data);
  const crumbs = $derived(document.ancestors.map((ancestor) => ({ label: ancestor.name, href: folderPath(slug, ancestor.number) })));
  const parentFolder = $derived(document.ancestors.at(-1) ?? null);
  const displayDate = $derived.by(() => {
    const date = pickSpaceDate(document.site.dateDisplay, document);
    return date === null ? null : dayjs(date).format('YYYY. M. D.');
  });
  const isPaginated = $derived(parseDocumentViewLayoutMode(document.layoutMode).type === 'paginated');

  $effect(() => {
    chrome.post = {
      title: document.title,
      url: document.url,
      eyebrow: parentFolder ? { label: parentFolder.name, href: folderPath(slug, parentFolder.number) } : null,
    };
    chrome.titleEl = titleEl ?? null;

    return () => {
      chrome.post = null;
      chrome.titleEl = null;
    };
  });

  const unlock = async (password: string) => {
    await unlockPublicationView({
      input: {
        publicationId: document.id,
        password,
      },
    });
  };
</script>

{#snippet head()}
  <div class={css({ paddingTop: { base: '32px', md: '48px' } })}>
    <div class={flex({ direction: 'column', width: 'full' })}>
      <a
        class={flex({
          alignItems: 'center',
          gap: '12px',
          minWidth: '0',
          width: 'fit',
          maxWidth: 'full',
          _hover: { '& [data-space-name]': { color: 'text.muted' } },
        })}
        href={spaceHomePath(slug)}
      >
        <Img
          style={css.raw({
            flexShrink: '0',
            size: '36px',
            borderRadius: '9px',
            objectFit: 'cover',
            boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.06)]',
          })}
          alt={`${document.site.name} 로고`}
          image$key={document.site.logo}
          size={96}
        />
        <span class={flex({ flexDirection: 'column', minWidth: '0' })}>
          <span
            class={css({ fontSize: '14px', fontWeight: 'semibold', lineHeight: '[1.4]', transition: 'colors', truncate: true })}
            data-space-name
          >
            {document.site.name}
          </span>
          {#if displayDate}
            <span class={css({ fontSize: '12px', lineHeight: '[1.45]', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>
              {displayDate}
            </span>
          {/if}
        </span>
      </a>

      {#if document.thumbnail}
        <div
          class={css({
            marginTop: '24px',
            aspectRatio: '[16 / 9]',
            borderRadius: '8px',
            backgroundColor: 'surface.canvas',
            overflow: 'hidden',
            isolation: 'isolate',
          })}
        >
          <Img
            style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
            alt={document.title}
            image$key={document.thumbnail}
            progressive
            size={1024}
          />
        </div>
      {/if}

      {#if crumbs.length > 0}
        <ReadingKicker style={css.raw({ marginTop: '28px' })} items={crumbs} />
      {/if}

      <div class={css({ marginTop: crumbs.length > 0 ? '12px' : '28px' })}>
        <ReadingTitle
          hasPassword={document.hasPassword}
          paginated={isPaginated}
          subtitle={document.subtitle}
          title={document.title}
          bind:titleEl
        >
          {#snippet actions()}
            <ShareLinkPopover style={css.raw({ marginLeft: '0', padding: '8px', borderRadius: '6px' })} href={document.url} iconSize={16} />
            <ReadingActionMenu document$key={document} />
          {/snippet}
        </ReadingTitle>
      </div>
    </div>
  </div>
{/snippet}

{#snippet foot()}
  <div
    class={flex({
      flexDirection: 'column',
      gap: '32px',
      paddingTop: '40px',
      paddingBottom: { base: '60px', lg: '80px' },
      width: 'full',
    })}
  >
    <div class={flex({ alignItems: 'center', flexWrap: 'wrap', gap: '6px', minWidth: '0' })}>
      {#each document.tags as tag (tag)}
        <TagChip name={tag} current={false} href={tagPath(slug, tag)} noscroll={false} />
      {/each}
      <ShareLinkPopover style={css.raw({ padding: '8px', borderRadius: '6px', marginRight: '-6px' })} href={document.url} iconSize={16} />
    </div>

    <ReadingReactions document$key={document} />

    <div class={css({ marginTop: '8px' })}>
      <PublicationSpaceCard publicationView$key={document} />
    </div>

    <div class={css({ marginTop: '4px' })}>
      <PublicationRecent publicationView$key={document} />
    </div>
  </div>
{/snippet}

<ReadingView
  document$key={document}
  {foot}
  {head}
  helmetTrailing={document.site.name}
  ogImageUrl={`${env.PUBLIC_API_URL}/og/p/${document.number}`}
  {unlock}
  user$key={user.data}
/>
