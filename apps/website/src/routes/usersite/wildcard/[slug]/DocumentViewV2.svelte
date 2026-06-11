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
  import { Editor as EditorComponent } from '$lib/editor-ffi/components';
  import { Editor, setupEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { registerLinkContextMenu } from '$lib/editor-ffi/handlers/link';
  import { unwrapError } from '$lib/graphql';
  import { graphql } from '$mearie';
  import BodyUnavailable from './BodyUnavailable.svelte';
  import ContentNavigation from './ContentNavigation.svelte';
  import DocumentActionMenu from './DocumentActionMenu.svelte';
  import DocumentEmojiReaction from './DocumentEmojiReaction.svelte';
  import DocumentViewSkeleton from './DocumentViewSkeleton.svelte';
  import ReadOnlyTouchSelectionSuppress from './ReadOnlyTouchSelectionSuppress.svelte';
  import ShareLinkPopover from './ShareLinkPopover.svelte';
  import type { UsersiteWildcardSlugPage_DocumentViewV2_entityView$key, UsersiteWildcardSlugPage_DocumentViewV2_user$key } from '$mearie';

  type Props = {
    entityView$key: UsersiteWildcardSlugPage_DocumentViewV2_entityView$key;
    user$key: UsersiteWildcardSlugPage_DocumentViewV2_user$key | null | undefined;
  };

  let { entityView$key, user$key }: Props = $props();

  const entityView = createFragment(
    graphql(`
      fragment UsersiteWildcardSlugPage_DocumentViewV2_entityView on EntityView {
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

        node {
          __typename

          ... on DocumentView {
            id
            title
            subtitle
            excerpt
            hasPassword
            protectContent
            allowReaction

            state {
              __typename
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

            reactions {
              id
              emoji
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

            ...Editor_document
            ...UsersiteWildcardSlugPage_DocumentEmojiReaction_documentView
          }
        }

        site {
          id
          name
          url

          logo {
            id
            ...Img_image
          }
        }

        ...UsersiteWildcardSlugPage_DocumentActionMenu_entityView
        ...UsersiteWildcardSlugPage_ContentNavigation_entityView
      }
    `),
    () => entityView$key,
  );

  const user = createFragment(
    graphql(`
      fragment UsersiteWildcardSlugPage_DocumentViewV2_user on User {
        id
      }
    `),
    () => user$key,
  );

  const [unlockDocumentView] = createMutation(
    graphql(`
      mutation UsersiteWildcardSlugPage_V2_UnlockDocumentView_Mutation($input: UnlockDocumentViewInput!) {
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

  const [verifyPersonalIdentity] = createMutation(
    graphql(`
      mutation UsersiteWildcardSlugPage_DocumentViewV2_VerifyPersonalIdentity_Mutation($input: VerifyPersonalIdentityInput!) {
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
      password: z.string(),
    }),
    onSubmit: async (data) => {
      if (entityView.data.node.__typename !== 'DocumentView') {
        return;
      }

      await unlockDocumentView({
        input: {
          documentId: entityView.data.node.id,
          password: data.password,
        },
      });

      mixpanel.track('unlock_document_view', {
        documentId: entityView.data.node.id,
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
  let destroyed = false;

  onMount(() => {
    hydrated = true;
  });

  const document = $derived(entityView.data.node.__typename === 'DocumentView' ? entityView.data.node : null);
  const documentId = $derived(document?.id);

  const graph = $derived(
    document?.documentBody?.__typename === 'DocumentViewBodyAvailableV2' ? Uint8Array.fromBase64(document.documentBody.graph) : undefined,
  );

  let createdForDocumentId: string | null = null;

  $effect(() => {
    const id = documentId;
    const g = graph;
    if (!id || !g) {
      if (ctx.editor) {
        ctx.editor.destroy();
        ctx.editor = undefined;
        createdForDocumentId = null;
      }
      return;
    }
    if (createdForDocumentId === id && ctx.editor) return;

    createdForDocumentId = id;
    const protectContent = document?.protectContent ?? false;

    untrack(async () => {
      const previous = ctx.editor;
      try {
        const editor = await Editor.create(g, { width: 1, height: 1, scale_factor: window.devicePixelRatio }, theme.currentThemeVariant);

        if (destroyed || createdForDocumentId !== id) {
          editor.destroy();
          return;
        }

        editor.readOnly = true;
        editor.protectContent = protectContent;
        ctx.editor = editor;
        previous?.destroy();
      } catch (err) {
        console.error(err);
      }
    });
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
    void documentId;
    editorReady = false;
  });

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

  const paginatedHeaderPaddingLeft = $derived.by(() => {
    const editor = ctx.editor;
    const layoutMode = editor?.rootAttrs?.layout_mode;
    if (!editor || layoutMode?.type !== 'paginated') return '0';
    return `${layoutMode.page_margin_left * editor.safeDisplayZoom()}px`;
  });

  const paginatedHeaderPaddingRight = $derived.by(() => {
    const editor = ctx.editor;
    const layoutMode = editor?.rootAttrs?.layout_mode;
    if (!editor || layoutMode?.type !== 'paginated') return '0';
    return `${layoutMode.page_margin_right * editor.safeDisplayZoom()}px`;
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

<svelte:head>
  <meta name="robots" content="noindex, nofollow" />
</svelte:head>

{#if document}
  <Helmet
    description={document.excerpt}
    image={{ size: 'large', src: `${env.PUBLIC_API_URL}/og/${entityView.data.id}` }}
    title={document.title}
  />

  {#if document.documentBody.__typename === 'DocumentViewBodyAvailableV2'}
    {#if graph}
      <ReadOnlyTouchSelectionSuppress enabled={ctx.editor?.gesture.gestureActive ?? false} />

      {#snippet documentHeader()}
        <div class={css({ paddingTop: { base: '48px', md: '80px' } })}>
          <div
            style:padding-left={paginatedHeaderPaddingLeft}
            style:padding-right={paginatedHeaderPaddingRight}
            class={flex({ direction: 'column', width: 'full' })}
          >
            <nav class={flex({ alignItems: 'center', gap: '6px', flexWrap: 'wrap', marginBottom: '20px' })}>
              <a class={flex({ alignItems: 'center', gap: '6px' })} href={entityView.data.site.url}>
                {#if entityView.data.site.logo}
                  <Img
                    style={css.raw({ size: '18px', borderRadius: '4px', objectFit: 'cover' })}
                    alt={`${entityView.data.site.name} 로고`}
                    image$key={entityView.data.site.logo}
                    size={24}
                  />
                {/if}
                <span class={css({ fontSize: '13px', color: 'text.faint', _hover: { color: 'text.muted' } })}>
                  {entityView.data.site.name}
                </span>
              </a>

              {#each entityView.data.ancestors as ancestor (ancestor.id)}
                {#if ancestor.node.__typename === 'FolderView'}
                  <span class={css({ fontSize: '13px', color: 'text.faint' })}>/</span>
                  <a class={css({ fontSize: '13px', color: 'text.faint', _hover: { color: 'text.muted' } })} href={`/${ancestor.slug}`}>
                    {ancestor.node.name}
                  </a>
                {/if}
              {/each}
            </nav>

            <div class={css({ fontSize: { base: '24px', lg: '28px' }, fontWeight: 'bold' })}>
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
                  borderColor: 'border.subtle',
                  backgroundColor: 'surface.subtle',
                  fontSize: '12px',
                  fontWeight: 'medium',
                  color: 'text.muted',
                })}
              >
                <Icon icon={LockOpenIcon} size={12} />
                <span>비밀번호 확인 후 열람 중</span>
              </div>
            {/if}

            <div class={flex({ align: 'center', justify: 'space-between', marginTop: '24px', paddingBottom: '16px' })}>
              <div class={flex({ align: 'center', gap: '8px', fontSize: '13px', color: 'text.faint' })}>
                {#if document.allowReaction && document.reactions.length > 0}
                  <div class={flex({ align: 'center', gap: '3px' })}>
                    <Icon icon={SmileIcon} />
                    <span>{comma(document.reactions.length)}</span>
                  </div>
                {/if}
              </div>

              <div class={flex({ align: 'center', marginLeft: 'auto', gap: '12px', color: 'text.muted' })}>
                <ShareLinkPopover href={entityView.data.url} />

                <DocumentActionMenu entityView$key={entityView.data} />
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
          <DocumentEmojiReaction documentView$key={document} />

          {#if document.protectContent}
            <div class={flex({ align: 'center', gap: '12px', marginLeft: 'auto', color: 'text.muted' })}>
              <ShareLinkPopover href={entityView.data.url} />

              <DocumentActionMenu entityView$key={entityView.data} />
            </div>
          {:else}
            <div class={flex({ align: 'center', marginLeft: 'auto' })}>
              <ShareLinkPopover href={entityView.data.url} />
            </div>
          {/if}
        </div>

        <div class={css({ paddingBottom: { base: '60px', lg: '80px' } })}>
          <ContentNavigation entityView$key={entityView.data} />
        </div>
      {/snippet}

      {#if hydrated && !editorReady}
        <div style:max-width="640px" style:padding-inline="20px" class={flex({ flexDirection: 'column', width: 'full', marginX: 'auto' })}>
          {@render documentHeader()}
          <DocumentViewSkeleton />
          {@render documentFooter()}
        </div>
      {/if}

      {#key document.id}
        <div class={flex({ flexDirection: 'column' })}>
          {#if document.protectContent}
            <ContentProtect>
              <EditorComponent active={false} document$key={document} onReady={() => (editorReady = true)} useWindowScroll>
                {#snippet header()}
                  {@render documentHeader()}
                {/snippet}

                {#snippet footer()}
                  {@render documentFooter()}
                {/snippet}
              </EditorComponent>
            </ContentProtect>
          {:else}
            <EditorComponent active={false} document$key={document} onReady={() => (editorReady = true)} useWindowScroll>
              {#snippet header()}
                {@render documentHeader()}
              {/snippet}

              {#snippet footer()}
                {@render documentFooter()}
              {/snippet}
            </EditorComponent>
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
                type="password"
                bind:value={form.fields.password}
              />

              {#if form.errors.password}
                <p class={css({ paddingLeft: '4px', fontSize: '12px', color: 'text.danger' })}>{form.errors.password}</p>
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
