<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import dayjs from 'dayjs';
  import { env } from '$env/dynamic/public';
  import { parseDocumentViewLayoutMode } from '$lib/usersite/document-view-layout';
  import ReadingActionMenu from '$lib/usersite/ReadingActionMenu.svelte';
  import ReadingKicker from '$lib/usersite/ReadingKicker.svelte';
  import ReadingReactions from '$lib/usersite/ReadingReactions.svelte';
  import ReadingTitle from '$lib/usersite/ReadingTitle.svelte';
  import ReadingView from '$lib/usersite/ReadingView.svelte';
  import ShareLinkPopover from '$lib/usersite/ShareLinkPopover.svelte';
  import { pickSiteDate } from '$lib/usersite/site-date';
  import { graphql } from '$mearie';
  import { getUsersiteChrome } from '../../../../chrome.svelte';
  import FolderCard from './FolderCard.svelte';
  import type { UsersiteApexSlugPage_DocumentViewV2_entityView$key, UsersiteApexSlugPage_DocumentViewV2_user$key } from '$mearie';

  type Props = {
    entityView$key: UsersiteApexSlugPage_DocumentViewV2_entityView$key;
    user$key: UsersiteApexSlugPage_DocumentViewV2_user$key | null | undefined;
  };

  let { entityView$key, user$key }: Props = $props();

  const chrome = getUsersiteChrome();
  let titleEl = $state<HTMLElement>();

  const entityView = createFragment(
    graphql(`
      fragment UsersiteApexSlugPage_DocumentViewV2_entityView on EntityView {
        id
        slug
        url

        ancestors {
          id
          slug

          node {
            __typename

            ... on FolderView {
              id
              name
            }
          }
        }

        site {
          id
          dateDisplay
        }

        node {
          __typename

          ... on DocumentView {
            id
            title
            subtitle
            hasPassword
            layoutMode
            createdAt
            updatedAt

            ...UsersiteReadingView_document
            ...UsersiteReadingReactions_document
            ...UsersiteReadingActionMenu_document
          }
        }

        ...UsersiteApexSlugPage_FolderCard_entityView
      }
    `),
    () => entityView$key,
  );

  const user = createFragment(
    graphql(`
      fragment UsersiteApexSlugPage_DocumentViewV2_user on User {
        id
        ...UsersiteReadingView_user
      }
    `),
    () => user$key,
  );

  const [unlockDocumentView] = createMutation(
    graphql(`
      mutation UsersiteApexSlugPage_V2_UnlockDocumentView_Mutation($input: UnlockDocumentViewInput!) {
        unlockDocumentView(input: $input) {
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
        }
      }
    `),
  );

  const document = $derived(entityView.data.node.__typename === 'DocumentView' ? entityView.data.node : null);
  const folderAncestors = $derived(
    entityView.data.ancestors
      .map((ancestor) => (ancestor.node.__typename === 'FolderView' ? { slug: ancestor.slug, name: ancestor.node.name } : null))
      .filter((ancestor) => ancestor !== null),
  );
  const parentFolder = $derived(folderAncestors.at(-1) ?? null);
  const displayDate = $derived.by(() => {
    if (!document) return null;
    const date = pickSiteDate(entityView.data.site.dateDisplay, document);
    return date === null ? null : dayjs(date).format('YYYY. M. D.');
  });
  const isPaginated = $derived(parseDocumentViewLayoutMode(document?.layoutMode).type === 'paginated');

  $effect(() => {
    if (!document) return;
    chrome.post = {
      title: document.title,
      url: entityView.data.url,
      eyebrow: parentFolder ? { label: parentFolder.name, href: `/s/${parentFolder.slug}` } : null,
    };
    chrome.titleEl = titleEl ?? null;

    return () => {
      chrome.post = null;
      chrome.titleEl = null;
    };
  });

  const unlock = async (password: string) => {
    if (!document) return;
    await unlockDocumentView({
      input: {
        documentId: document.id,
        password,
      },
    });
  };
</script>

<svelte:head>
  <meta name="robots" content="noindex, nofollow" />
</svelte:head>

{#if document}
  {#snippet head()}
    <div class={css({ paddingTop: { base: '32px', md: '48px' } })}>
      <div class={flex({ direction: 'column', width: 'full' })}>
        {#if displayDate}
          <span class={css({ fontSize: '12px', lineHeight: '[1.45]', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>
            {displayDate}
          </span>
        {/if}

        {#if parentFolder}
          <ReadingKicker
            style={css.raw({ marginTop: displayDate ? '28px' : '0' })}
            items={folderAncestors.map((folder) => ({ label: folder.name, href: `/s/${folder.slug}` }))}
          />
        {/if}

        <div class={css({ marginTop: parentFolder ? '12px' : '28px' })}>
          <ReadingTitle
            hasPassword={document.hasPassword}
            paginated={isPaginated}
            subtitle={document.subtitle}
            title={document.title}
            bind:titleEl
          >
            {#snippet actions()}
              <ShareLinkPopover
                style={css.raw({ marginLeft: '0', padding: '8px', borderRadius: '6px' })}
                href={entityView.data.url}
                iconSize={16}
              />
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
      <div class={flex({ alignItems: 'center', justifyContent: 'flex-end', minWidth: '0' })}>
        <ShareLinkPopover
          style={css.raw({ padding: '8px', borderRadius: '6px', marginRight: '-6px' })}
          href={entityView.data.url}
          iconSize={16}
        />
      </div>

      <ReadingReactions document$key={document} />

      <div class={css({ marginTop: '8px' })}>
        <FolderCard entityView$key={entityView.data} />
      </div>
    </div>
  {/snippet}

  <ReadingView
    document$key={document}
    {foot}
    {head}
    ogImageUrl={`${env.PUBLIC_API_URL}/og/${entityView.data.id}`}
    {unlock}
    user$key={user.data}
  />
{/if}
