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
  import { comma, serializeOAuthState } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import qs from 'query-string';
  import { onDestroy, onMount, untrack } from 'svelte';
  import { z } from 'zod';
  import LockIcon from '~icons/lucide/lock';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import ShieldAlertIcon from '~icons/lucide/shield-alert';
  import SmileIcon from '~icons/lucide/smile';
  import { page } from '$app/state';
  import { env } from '$env/dynamic/public';
  import { Img } from '$lib/components';
  import { Editor as EditorComponent, EditorFailureOverlay } from '$lib/editor-ffi/components';
  import { Editor, setupEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { registerLinkContextMenu } from '$lib/editor-ffi/handlers/link';
  import { browserScaleFactor } from '$lib/editor-ffi/zoom';
  import { unwrapError } from '$lib/graphql';
  import BodyUnavailable from '$lib/usersite/BodyUnavailable.svelte';
  import DocumentDomMirror from '$lib/usersite/DocumentDomMirror.svelte';
  import DocumentViewSkeleton from '$lib/usersite/DocumentViewSkeleton.svelte';
  import ReadOnlyTouchSelectionSuppress from '$lib/usersite/ReadOnlyTouchSelectionSuppress.svelte';
  import ShareLinkPopover from '$lib/usersite/ShareLinkPopover.svelte';
  import { graphql } from '$mearie';
  import { getUsersiteChrome } from '../../../chrome.svelte';
  import CollectionNavigation from './CollectionNavigation.svelte';
  import PublicationActionMenu from './PublicationActionMenu.svelte';
  import PublicationEmojiReaction from './PublicationEmojiReaction.svelte';
  import type {
    UsersiteWildcardPublicationPage_PublicationViewV2_publicationView$key,
    UsersiteWildcardPublicationPage_PublicationViewV2_user$key,
  } from '$mearie';

  type Props = {
    publicationView$key: UsersiteWildcardPublicationPage_PublicationViewV2_publicationView$key;
    user$key: UsersiteWildcardPublicationPage_PublicationViewV2_user$key | null | undefined;
  };

  type EditorFailure = {
    documentId: string;
    editor?: Editor;
  };

  let { publicationView$key, user$key }: Props = $props();

  const chrome = getUsersiteChrome();
  let titleEl = $state<HTMLElement>();

  const publication = createFragment(
    graphql(`
      fragment UsersiteWildcardPublicationPage_PublicationViewV2_publicationView on PublicationView {
        id
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

        collection {
          id
          name
        }

        reactions {
          id
          emoji
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
        ...UsersiteWildcardPublicationPage_PublicationEmojiReaction_publicationView
        ...UsersiteWildcardPublicationPage_PublicationActionMenu_publicationView
        ...UsersiteWildcardPublicationPage_CollectionNavigation_publicationView
      }
    `),
    () => publicationView$key,
  );

  const user = createFragment(
    graphql(`
      fragment UsersiteWildcardPublicationPage_PublicationViewV2_user on User {
        id
      }
    `),
    () => user$key,
  );

  const [unlockPublicationView] = createMutation(
    graphql(`
      mutation UsersiteWildcardPublicationPage_PublicationViewV2_UnlockPublicationView_Mutation($input: UnlockPublicationViewInput!) {
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
      mutation UsersiteWildcardPublicationPage_PublicationViewV2_VerifyPersonalIdentity_Mutation($input: VerifyPersonalIdentityInput!) {
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

  let hydrated = $state(false);
  let editorReady = $state(false);
  let editorFailure = $state<EditorFailure>();
  let editorForDocumentId = $state<string | null>(null);
  let fallbackBodySurface = $state<HTMLDivElement>();
  let destroyed = false;

  onMount(() => {
    hydrated = true;
  });

  const document = $derived(publication.data);

  $effect(() => {
    if (!document) return;
    chrome.post = {
      title: document.title,
      url: document.url,
      collection: document.collection ? { id: document.collection.id, name: document.collection.name } : null,
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
        editor.imageAssets.set(asset.id, {
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

  const isPaginated = $derived(ctx.editor?.rootAttrs?.layout_mode.type === 'paginated');

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
    image={{ size: 'large', src: `${env.PUBLIC_API_URL}/og/p/${publication.data.id}` }}
    title={document.title}
    trailing={document.space.name}
  />

  {#if document.documentBody.__typename === 'DocumentViewBodyAvailableV2'}
    {#if graph}
      <ReadOnlyTouchSelectionSuppress enabled={ctx.editor?.gesture.gestureActive ?? false} />

      {#snippet documentHeader()}
        <div class={css({ paddingTop: { base: '48px', md: '80px' } })}>
          <div class={flex({ direction: 'column', width: 'full' })}>
            <nav class={flex({ alignItems: 'center', gap: '6px', flexWrap: 'wrap', marginBottom: '20px' })}>
              <a class={flex({ alignItems: 'center', gap: '6px' })} href="/">
                <Img
                  style={css.raw({ size: '18px', borderRadius: '4px', objectFit: 'cover' })}
                  alt={`${document.space.name} 로고`}
                  image$key={document.space.logo}
                  size={24}
                />
                <span class={css({ fontSize: '13px', color: 'text.hint', _hover: { color: 'text.muted' } })}>
                  {document.space.name}
                </span>
              </a>

              {#if document.collection}
                <span class={css({ fontSize: '13px', color: 'text.hint' })}>/</span>
                <a
                  class={css({ fontSize: '13px', color: 'text.hint', _hover: { color: 'text.muted' } })}
                  href={`/s/${document.collection.id}`}
                >
                  {document.collection.name}
                </a>
              {/if}
            </nav>

            <div bind:this={titleEl} class={css({ fontSize: { base: '24px', lg: '28px' }, fontWeight: 'bold' })}>
              {document.title}
            </div>

            {#if document.subtitle}
              <div class={css({ marginTop: '8px', fontSize: { base: '14px', lg: '16px' }, fontWeight: 'medium' })}>
                {document.subtitle}
              </div>
            {/if}

            {#if document.hasPassword}
              <div
                class={flex({
                  alignItems: 'center',
                  gap: '4px',
                  marginTop: document.subtitle ? '10px' : '12px',
                  width: 'fit',
                  paddingX: '8px',
                  paddingY: '4px',
                  borderRadius: 'full',
                  borderWidth: '1px',
                  borderColor: 'border.hairline',
                  backgroundColor: 'surface.canvas',
                  fontSize: '12px',
                  fontWeight: 'medium',
                  color: 'text.muted',
                })}
              >
                <Icon icon={LockOpenIcon} size={12} />
                <span>비밀번호 확인 후 열람 중</span>
              </div>
            {/if}

            {#if document.tags.length > 0}
              <div class={flex({ flexWrap: 'wrap', gap: '6px', marginTop: '12px' })}>
                {#each document.tags as tag (tag)}
                  <a
                    class={css({
                      paddingX: '8px',
                      paddingY: '4px',
                      borderRadius: 'full',
                      borderWidth: '1px',
                      borderColor: 'border.hairline',
                      backgroundColor: 'surface.canvas',
                      fontSize: '12px',
                      fontWeight: 'medium',
                      color: 'text.muted',
                      _hover: { color: 'text.default' },
                    })}
                    href={`/t/${encodeURIComponent(tag)}`}
                  >
                    {tag}
                  </a>
                {/each}
              </div>
            {/if}

            <div class={flex({ align: 'center', justify: 'space-between', marginTop: '24px', paddingBottom: '16px' })}>
              <div class={flex({ align: 'center', gap: '8px', fontSize: '13px', color: 'text.hint' })}>
                {#if document.allowReaction && document.reactions.length > 0}
                  <div class={flex({ align: 'center', gap: '3px' })}>
                    <Icon icon={SmileIcon} />
                    <span>{comma(document.reactions.length)}</span>
                  </div>
                {/if}

                {#if document.space.dateDisplay !== 'NONE'}
                  <span>
                    {dayjs(document.space.dateDisplay === 'PUBLISHED_AT' ? document.publishedAt : document.updatedAt).format('YYYY. M. D.')}
                  </span>
                {/if}
              </div>

              <div class={flex({ align: 'center', marginLeft: 'auto', gap: '12px', color: 'text.muted' })}>
                <ShareLinkPopover href={document.url} />

                <PublicationActionMenu publicationView$key={document} />
              </div>
            </div>

            {#if !isPaginated}
              <HorizontalDivider style={css.raw({ marginBottom: '24px' })} />
            {/if}
          </div>
        </div>
      {/snippet}

      {#snippet documentFooter()}
        <div
          class={flex({
            align: 'flex-start',
            justify: 'space-between',
            gap: '8px',
            marginTop: '20px',
            paddingBottom: '10px',
            width: 'full',
          })}
        >
          <PublicationEmojiReaction publicationView$key={document} />

          {#if document.protectContent}
            <div class={flex({ align: 'center', gap: '12px', marginLeft: 'auto', color: 'text.muted' })}>
              <ShareLinkPopover href={document.url} />

              <PublicationActionMenu publicationView$key={document} />
            </div>
          {:else}
            <div class={flex({ align: 'center', marginLeft: 'auto' })}>
              <ShareLinkPopover href={document.url} />
            </div>
          {/if}
        </div>

        <div class={css({ paddingBottom: { base: '60px', lg: '80px' } })}>
          <CollectionNavigation publicationView$key={document} />
        </div>
      {/snippet}

      <div class={css({ position: 'relative', isolation: 'isolate' })}>
        {#if hydrated && !editorReady}
          <div
            class={css({
              position: 'absolute',
              top: '0',
              left: '0',
              right: '0',
              zIndex: 'editorOverlay',
              minHeight: '[100dvh]',
              backgroundColor: 'surface.default',
            })}
          >
            <div
              style:max-width="640px"
              style:padding-inline="20px"
              class={flex({ flexDirection: 'column', width: 'full', marginX: 'auto' })}
            >
              {@render documentHeader()}
              <div bind:this={fallbackBodySurface}>
                <DocumentViewSkeleton />
              </div>
              {@render documentFooter()}
            </div>
          </div>
        {/if}

        {#key document.id}
          <div class={flex({ flexDirection: 'column' })}>
            {#if document.protectContent}
              <ContentProtect>
                <EditorComponent active={false} document$key={document} onReady={handleEditorReady} useWindowScroll>
                  {#snippet header()}
                    {@render documentHeader()}
                  {/snippet}

                  {#snippet footer()}
                    {@render documentFooter()}
                  {/snippet}
                </EditorComponent>
              </ContentProtect>
            {:else}
              <EditorComponent active={false} document$key={document} onReady={handleEditorReady} useWindowScroll>
                {#snippet header()}
                  {@render documentHeader()}
                {/snippet}

                {#snippet footer()}
                  {@render documentFooter()}
                {/snippet}
              </EditorComponent>
            {/if}
          </div>

          <DocumentDomMirror
            editor={editorReady && editorForDocumentId === document.id ? ctx.editor : undefined}
            excerpt={document.excerpt}
          />
        {/key}

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
