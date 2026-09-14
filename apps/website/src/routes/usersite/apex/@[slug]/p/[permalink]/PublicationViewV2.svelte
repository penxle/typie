<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import * as PortOne from '@portone/browser-sdk/v2';
  import { TypieError } from '@typie/lib/errors';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, ContentProtect, Helmet, HorizontalDivider, Icon, TextInput } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import { createForm, FormError } from '@typie/ui/form';
  import { Toast } from '@typie/ui/notification';
  import { serializeOAuthState } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import qs from 'query-string';
  import { onDestroy, untrack } from 'svelte';
  import { z } from 'zod';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import LockIcon from '~icons/lucide/lock';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import ShieldAlertIcon from '~icons/lucide/shield-alert';
  import { page } from '$app/state';
  import { env } from '$env/dynamic/public';
  import { Img } from '$lib/components';
  import { Editor as EditorComponent, EditorFailureOverlay } from '$lib/editor-ffi/components';
  import { Editor, setupEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { registerLinkContextMenu } from '$lib/editor-ffi/handlers/link';
  import { browserScaleFactor } from '$lib/editor-ffi/zoom';
  import { unwrapError } from '$lib/graphql';
  import BodyUnavailable from '$lib/usersite/BodyUnavailable.svelte';
  import { parseDocumentViewLayoutMode } from '$lib/usersite/document-view-layout';
  import DocumentDomMirror from '$lib/usersite/DocumentDomMirror.svelte';
  import DocumentViewFrame from '$lib/usersite/DocumentViewFrame.svelte';
  import ReadOnlyTouchSelectionSuppress from '$lib/usersite/ReadOnlyTouchSelectionSuppress.svelte';
  import ShareLinkPopover from '$lib/usersite/ShareLinkPopover.svelte';
  import { graphql } from '$mearie';
  import { getUsersiteChrome } from '../../../../chrome.svelte';
  import { currentSpaceSlug } from '../../current-space-slug';
  import { seriesPath, spaceHomePath, tagPath } from '../../paths';
  import TagChip from '../../TagChip.svelte';
  import PublicationActionMenu from './PublicationActionMenu.svelte';
  import PublicationReactions from './PublicationReactions.svelte';
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

  type EditorFailure = {
    documentId: string;
    editor?: Editor;
  };

  let { publicationView$key, user$key }: Props = $props();

  const chrome = getUsersiteChrome();
  const slug = $derived(currentSpaceSlug());
  let titleEl = $state<HTMLElement>();

  const publication = createFragment(
    graphql(`
      fragment UsersiteSpacePublicationPage_PublicationViewV2_publicationView on PublicationView {
        id
        permalink
        documentId
        title
        subtitle
        excerpt
        hasPassword
        protectContent
        allowReaction
        publishedAt
        updatedAt
        tags
        url
        layoutMode

        collection {
          id
          permalink
          name

          publications {
            id
          }
        }

        thumbnail {
          id
          ...Img_image
        }

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

        space {
          id
          name
          dateDisplay

          logo {
            id
            ...Img_image
          }
        }

        ...Editor_document
        ...UsersiteSpacePublicationPage_PublicationActionMenu_publicationView
        ...UsersiteSpacePublicationPage_PublicationReactions_publicationView
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

  const [verifyPersonalIdentity] = createMutation(
    graphql(`
      mutation UsersiteSpacePublicationPage_PublicationViewV2_VerifyPersonalIdentity_Mutation($input: VerifyPersonalIdentityInput!) {
        verifyPersonalIdentity(input: $input) {
          id

          personalIdentity {
            id
            expiresAt
          }
        }
      }
    `),
  );

  const form = createForm({
    schema: z.object({
      password: z.string().trim(),
    }),
    onSubmit: async (data) => {
      await unlockPublicationView({
        input: {
          publicationId: publication.data.id,
          password: data.password,
        },
      });

      mixpanel.track('unlock_document_view', {
        documentId: publication.data.documentId,
      });
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'invalid_password') {
        throw new FormError('password', '비밀번호가 올바르지 않습니다.');
      }
    },
  });

  $effect(() => {
    void form;
  });

  const theme = getThemeContext();
  const ctx = setupEditorContext();

  let editorReady = $state(false);
  let editorFailure = $state<EditorFailure>();
  let editorForDocumentId = $state<string | null>(null);
  let fallbackBodySurface = $state<HTMLDivElement>();
  let destroyed = false;

  const document = $derived(publication.data);
  const seriesPosition = $derived(
    document.collection ? document.collection.publications.findIndex((item) => item.id === document.id) + 1 : 0,
  );
  const displayDate = $derived(
    document.space.dateDisplay === 'NONE'
      ? null
      : dayjs(document.space.dateDisplay === 'PUBLISHED_AT' ? document.publishedAt : document.updatedAt).format('YYYY. M. D.'),
  );

  $effect(() => {
    if (!document) return;
    chrome.post = {
      title: document.title,
      url: document.url,
      collection: document.collection ? { permalink: document.collection.permalink, name: document.collection.name } : null,
    };
    chrome.titleEl = titleEl ?? null;

    return () => {
      chrome.post = null;
      chrome.titleEl = null;
    };
  });
  const editorKey = $derived(document.id);
  const activeEditorFailure = $derived(editorFailure?.documentId === editorKey ? editorFailure : undefined);
  const failureSurface = $derived(activeEditorFailure ? (activeEditorFailure.editor?.extensionAreaEl ?? fallbackBodySurface) : undefined);

  const graph = $derived(
    document?.documentBody?.__typename === 'DocumentViewBodyAvailableV2' ? Uint8Array.fromBase64(document.documentBody.graph) : undefined,
  );

  let createdForDocumentId: string | null = null;

  const setEditorFailure = (id: string, editor?: Editor) => {
    if (destroyed || editorKey !== id || editorFailure?.documentId === id) return;
    editorFailure = { documentId: id, editor };
  };

  $effect(() => {
    const id = editorKey;
    const g = graph;
    if (!id || !g) {
      ctx.editor?.destroy();
      ctx.editor = undefined;
      createdForDocumentId = null;
      editorForDocumentId = null;
      editorReady = false;
      editorFailure = undefined;
      return;
    }
    if (createdForDocumentId === id) return;

    createdForDocumentId = id;
    ctx.editor?.destroy();
    ctx.editor = undefined;
    editorForDocumentId = null;
    const protectContent = document?.protectContent ?? false;

    untrack(async () => {
      try {
        const editor = await Editor.create(g, { width: 1, height: 1, scale_factor: browserScaleFactor() }, theme.currentThemeVariant);

        if (destroyed || createdForDocumentId !== id) {
          editor.destroy();
          return;
        }

        editor.readOnly = true;
        editor.protectContent = protectContent;
        editorForDocumentId = id;
        ctx.editor = editor;
      } catch (err) {
        if (destroyed || createdForDocumentId !== id || editorKey !== id) return;
        console.error(err);
        setEditorFailure(id);
      }
    });
  });

  $effect(() => {
    const editor = ctx.editor;
    const id = editorForDocumentId;
    const failure = editor?.failure;
    if (editor && id && failure !== undefined) {
      setEditorFailure(id, editor);
    }
  });

  $effect(() => {
    const surface = failureSurface;
    if (!surface) return;
    surface.inert = true;
    return () => {
      surface.inert = false;
    };
  });

  $effect(() => {
    if (ctx.editor) {
      ctx.editor.protectContent = document?.protectContent ?? false;
    }
  });

  $effect(() => {
    const editor = ctx.editor;
    if (!editor) return;
    return registerLinkContextMenu(editor);
  });

  $effect(() => {
    void editorKey;
    editorReady = false;
    editorFailure = undefined;
  });

  const handleEditorReady = () => {
    if (editorForDocumentId === editorKey && ctx.editor) {
      editorReady = true;
    }
  };

  const assets = $derived(document?.assets);

  $effect(() => {
    const editor = ctx.editor;
    if (!editor || !assets) return;

    for (const asset of assets) {
      if (asset.__typename === 'Image') {
        editor.images.assets.set(asset.id, {
          id: asset.id,
          url: asset.url,
          originalUrl: asset.originalUrl,
          width: asset.width,
          height: asset.height,
          placeholder: asset.placeholder,
        });
      } else if (asset.__typename === 'File') {
        ctx.fileAssets.set(asset.id, {
          id: asset.id,
          url: asset.url,
          name: asset.name,
          size: asset.size,
        });
      } else if (asset.__typename === 'Embed') {
        editor.embedAssets.set(asset.id, {
          id: asset.id,
          url: asset.url,
          title: asset.title ?? null,
          description: asset.description ?? null,
          thumbnailUrl: asset.thumbnailUrl ?? null,
          html: asset.html ?? null,
        });
      } else if (asset.__typename === 'DocumentArchivedNode') {
        editor.archivedAssets.set(asset.id, {
          id: asset.id,
          content: asset.content,
        });
      }
    }
  });

  const layoutMode = $derived(parseDocumentViewLayoutMode(document.layoutMode));
  const isPaginated = $derived(layoutMode.type === 'paginated');

  const authorizeUrl = $derived(
    qs.stringifyUrl({
      url: `${env.PUBLIC_AUTH_URL}/authorize`,
      query: {
        client_id: env.PUBLIC_OIDC_CLIENT_ID,
        response_type: 'code',
        redirect_uri: `${page.url.origin}/authorize`,
        state: serializeOAuthState({ redirect_uri: page.url.href }),
      },
    }),
  );

  const handleVerification = async () => {
    try {
      mixpanel.track('verify_personal_identity_start');
      sessionStorage.setItem('redirect_uri', page.url.href);

      const resp = await PortOne.requestIdentityVerification({
        storeId: 'store-e1e69136-38bb-42dd-b226-3c78e03c1ff1',
        identityVerificationId: `identity-verification-${nanoid()}`,
        channelKey: 'channel-key-31e03361-26cb-4810-86ed-801cce4f570f',
        redirectUrl: `${page.url.origin}/identity`,
      });

      if (resp === undefined) {
        return;
      }

      await verifyPersonalIdentity({
        input: {
          identityVerificationId: resp.identityVerificationId,
        },
      });

      mixpanel.track('verify_personal_identity_success');
      location.reload();
    } catch (err) {
      const errorMessages: Record<string, string> = {
        identity_verification_failed: '인증에 실패했습니다.',
        same_identity_exists: '이미 다른 계정에 인증된 정보입니다.',
      };

      const error = unwrapError(err);
      if (error instanceof TypieError) {
        const message = errorMessages[error.code] || error.code;
        Toast.error(message);
      }
    }
  };

  onDestroy(() => {
    destroyed = true;
    ctx.editor?.destroy();
    ctx.editor = undefined;
  });
</script>

{#if document}
  <Helmet
    description={document.excerpt}
    image={{ size: 'large', src: `${env.PUBLIC_API_URL}/og/p/${publication.data.permalink}` }}
    title={document.title}
    trailing={document.space.name}
  />

  {#if document.documentBody.__typename === 'DocumentViewBodyAvailableV2'}
    {#if graph}
      <ReadOnlyTouchSelectionSuppress enabled={ctx.editor?.gesture.gestureActive ?? false} />

      {#snippet documentHeader()}
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
                alt={`${document.space.name} 로고`}
                image$key={document.space.logo}
                size={96}
              />
              <span class={flex({ flexDirection: 'column', minWidth: '0' })}>
                <span
                  class={css({ fontSize: '14px', fontWeight: 'semibold', lineHeight: '[1.4]', transition: 'colors', truncate: true })}
                  data-space-name
                >
                  {document.space.name}
                </span>
                {#if displayDate || seriesPosition > 0}
                  <span
                    class={flex({
                      alignItems: 'center',
                      gap: '6px',
                      fontSize: '12px',
                      lineHeight: '[1.45]',
                      color: 'text.hint',
                      fontVariantNumeric: 'tabular-nums',
                      whiteSpace: 'nowrap',
                    })}
                  >
                    {#if displayDate}
                      <span>{displayDate}</span>
                    {/if}
                    {#if displayDate && seriesPosition > 0}
                      <i
                        class={css({ flexShrink: '0', size: '2px', borderRadius: 'full', backgroundColor: 'border.emphasis' })}
                        aria-hidden="true"
                      ></i>
                    {/if}
                    {#if seriesPosition > 0}
                      <span>{seriesPosition}번째 글</span>
                    {/if}
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

            {#if document.collection}
              <a
                class={flex({
                  alignItems: 'center',
                  gap: '2px',
                  width: 'fit',
                  maxWidth: 'full',
                  marginTop: '28px',
                  fontSize: '13px',
                  fontWeight: 'medium',
                  color: 'text.muted',
                  transition: 'colors',
                  _hover: { color: 'text.default' },
                })}
                href={seriesPath(slug, document.collection.permalink)}
              >
                <span class={css({ truncate: true })}>{document.collection.name}</span>
                <Icon icon={ChevronRightIcon} size={14} />
              </a>
            {/if}

            <div
              class={flex({
                alignItems: 'flex-start',
                justifyContent: 'space-between',
                gap: '16px',
                marginTop: document.collection ? '12px' : '28px',
              })}
            >
              <div class={flex({ flexDirection: 'column', minWidth: '0' })}>
                <h1
                  bind:this={titleEl}
                  class={css({
                    fontSize: { base: '26px', md: '32px' },
                    fontWeight: 'bold',
                    lineHeight: '[1.35]',
                    letterSpacing: '-0.02em',
                    textWrap: 'balance',
                  })}
                >
                  {document.title}
                </h1>

                {#if document.subtitle}
                  <p
                    class={css({
                      marginTop: '8px',
                      fontSize: { base: '15px', md: '17px' },
                      fontWeight: 'medium',
                      lineHeight: '[1.5]',
                      color: 'text.muted',
                    })}
                  >
                    {document.subtitle}
                  </p>
                {/if}

                {#if document.hasPassword}
                  <div
                    class={flex({
                      alignItems: 'center',
                      gap: '5px',
                      marginTop: '16px',
                      width: 'fit',
                      height: '26px',
                      paddingX: '10px',
                      borderRadius: 'full',
                      borderWidth: '1px',
                      borderColor: 'border.hairline',
                      fontSize: '12px',
                      color: 'text.muted',
                    })}
                  >
                    <Icon icon={LockOpenIcon} size={12} />
                    <span>비밀번호 확인 후 열람 중</span>
                  </div>
                {/if}
              </div>

              <div
                class={flex({
                  flexShrink: '0',
                  alignItems: 'center',
                  gap: '2px',
                  marginTop: '4px',
                  marginRight: '-6px',
                  color: 'text.muted',
                })}
              >
                <ShareLinkPopover
                  style={css.raw({ marginLeft: '0', padding: '8px', borderRadius: '6px' })}
                  href={document.url}
                  iconSize={16}
                />
                <PublicationActionMenu publicationView$key={document} />
              </div>
            </div>

            {#if isPaginated}
              <div class={css({ height: '32px' })}></div>
            {:else}
              <HorizontalDivider style={css.raw({ marginTop: '32px', marginBottom: '32px' })} color="secondary" />
            {/if}
          </div>
        </div>
      {/snippet}

      {#snippet documentFooter()}
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
            <ShareLinkPopover
              style={css.raw({ padding: '8px', borderRadius: '6px', marginRight: '-6px' })}
              href={document.url}
              iconSize={16}
            />
          </div>

          <PublicationReactions publicationView$key={document} />

          <div class={css({ marginTop: '8px' })}>
            <PublicationSpaceCard publicationView$key={document} />
          </div>

          <div class={css({ marginTop: '4px' })}>
            <PublicationRecent publicationView$key={document} />
          </div>
        </div>
      {/snippet}

      {#key document.id}
        <div class={css({ position: 'relative', isolation: 'isolate' })}>
          <DocumentViewFrame {layoutMode} ready={editorReady} bind:bodySurface={fallbackBodySurface}>
            {#snippet header()}
              {@render documentHeader()}
            {/snippet}

            {#snippet footer()}
              {@render documentFooter()}
            {/snippet}

            {#if document.protectContent}
              <ContentProtect>
                <EditorComponent
                  style={css.raw({ paddingBottom: isPaginated ? '40px' : '0' })}
                  active={false}
                  document$key={document}
                  onReady={handleEditorReady}
                  useWindowScroll
                />
              </ContentProtect>
            {:else}
              <EditorComponent
                style={css.raw({ paddingBottom: isPaginated ? '40px' : '0' })}
                active={false}
                document$key={document}
                onReady={handleEditorReady}
                useWindowScroll
              />
            {/if}
          </DocumentViewFrame>

          <DocumentDomMirror
            editor={editorReady && editorForDocumentId === document.id ? ctx.editor : undefined}
            excerpt={document.excerpt}
          />

          {#if activeEditorFailure && failureSurface}
            <EditorFailureOverlay
              id={`usersite-editor-${document.id}`}
              actionLabel="새로고침"
              contentPosition="viewport"
              onAction={() => location.reload()}
              surfaceElement={failureSurface}
            />
          {/if}
        </div>
      {/key}
    {/if}
  {:else if document.documentBody.__typename === 'DocumentViewBodyUnavailable'}
    <div class={flex({ align: 'center', justify: 'center', minHeight: '[100dvh]', fontSize: '16px', fontWeight: 'medium' })}>
      {#if document.documentBody.reason === 'REQUIRE_IDENTITY_VERIFICATION'}
        <BodyUnavailable description="본인 인증이 필요한 글이에요" icon={ShieldAlertIcon} title="연령제한글">
          {#if user.data}
            <Button style={css.raw({ width: 'full' })} onclick={handleVerification} variant="secondary">본인 인증</Button>
          {:else}
            <Button style={css.raw({ width: 'full' })} external href={authorizeUrl} type="link" variant="secondary">
              로그인 후 본인 인증하기
            </Button>
          {/if}
        </BodyUnavailable>
      {:else if document.documentBody.reason === 'REQUIRE_MINIMUM_AGE'}
        <BodyUnavailable
          description="이 글은 연령 기준에 따라 현재 계정으로는 열람이 제한되어 있어요"
          icon={ShieldAlertIcon}
          title="연령제한글"
        />
      {:else if document.documentBody.reason === 'REQUIRE_PASSWORD'}
        <form onsubmit={form.handleSubmit}>
          <BodyUnavailable description="해당 내용은 비밀번호 입력이 필요해요" icon={LockIcon} title="비밀글">
            <div class={flex({ direction: 'column', gap: '4px' })}>
              <TextInput
                id="password"
                style={css.raw({ width: 'full', height: '36px' })}
                placeholder="비밀번호를 입력하세요"
                type="text"
                bind:value={form.fields.password}
              />

              {#if form.errors.password}
                <p class={css({ paddingLeft: '4px', fontSize: '12px', color: 'danger.default' })}>{form.errors.password}</p>
              {/if}
            </div>

            <Button style={css.raw({ marginTop: '8px', width: 'full' })} type="submit">확인</Button>
          </BodyUnavailable>
        </form>
      {:else}
        {document.documentBody.reason}
      {/if}
    </div>
  {/if}
{/if}
