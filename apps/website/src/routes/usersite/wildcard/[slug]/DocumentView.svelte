<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import * as PortOne from '@portone/browser-sdk/v2';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, ContentProtect, Helmet, HorizontalDivider, Icon, TextInput } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import { Toast } from '@typie/ui/notification';
  import { comma, serializeOAuthState } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import qs from 'query-string';
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import LockIcon from '~icons/lucide/lock';
  import ShieldAlertIcon from '~icons/lucide/shield-alert';
  import SmileIcon from '~icons/lucide/smile';
  import { page } from '$app/state';
  import { env } from '$env/dynamic/public';
  import { Img } from '$lib/components';
  import { Editor as EditorComponent } from '$lib/components/editor';
  import { setupEditorContext } from '$lib/editor/context.svelte';
  import { Editor } from '$lib/editor/editor.svelte';
  import { unwrapError } from '$lib/graphql';
  import { initWasm } from '$lib/wasm';
  import { graphql } from '$mearie';
  import ContentNavigation from './ContentNavigation.svelte';
  import DocumentActionMenu from './DocumentActionMenu.svelte';
  import DocumentEmojiReaction from './DocumentEmojiReaction.svelte';
  import DocumentViewSkeleton from './DocumentViewSkeleton.svelte';
  import PostViewBodyUnavailable from './PostViewBodyUnavailable.svelte';
  import ReadOnlyTouchSelectionSuppress from './ReadOnlyTouchSelectionSuppress.svelte';
  import ShareLinkPopover from './ShareLinkPopover.svelte';
  import type { UsersiteWildcardSlugPage_DocumentView_entityView$key, UsersiteWildcardSlugPage_DocumentView_user$key } from '$mearie';

  type Props = {
    entityView$key: UsersiteWildcardSlugPage_DocumentView_entityView$key;
    user$key: UsersiteWildcardSlugPage_DocumentView_user$key | null | undefined;
  };

  let { entityView$key, user$key }: Props = $props();

  const entityView = createFragment(
    graphql(`
      fragment UsersiteWildcardSlugPage_DocumentView_entityView on EntityView {
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
            snapshot
            hasPassword
            protectContent
            allowReaction

            documentBody: body {
              __typename

              ... on DocumentViewBodyAvailable {
                snapshot
              }

              ... on DocumentViewBodyUnavailable {
                reason
              }
            }

            reactions {
              id
              emoji
            }

            fontFamilies {
              id
              familyName
              displayName
              state

              fonts {
                id
                weight
                subfamilyDisplayName
                url
                state
              }
            }

            assets {
              __typename

              ... on Image {
                id
                url
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
            }

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
      fragment UsersiteWildcardSlugPage_DocumentView_user on User {
        id
      }
    `),
    () => user$key,
  );

  const [unlockDocumentView] = createMutation(
    graphql(`
      mutation UsersiteWildcardSlugPage_UnlockDocumentView_Mutation($input: UnlockDocumentViewInput!) {
        unlockDocumentView(input: $input) {
          id

          documentBody: body {
            __typename

            ... on DocumentViewBodyAvailable {
              snapshot
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
      mutation UsersiteWildcardSlugPage_DocumentView_VerifyPersonalIdentity_Mutation($input: VerifyPersonalIdentityInput!) {
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

  const ctx = setupEditorContext();
  const editor = new Editor();
  editor.contentReady = true;
  ctx.editor = editor;

  let hydrated = $state(false);
  let editorReady = $state(false);

  onMount(() => {
    hydrated = true;
  });

  const document = $derived(entityView.data.node.__typename === 'DocumentView' ? entityView.data.node : null);

  $effect(() => {
    void document?.id;
    editorReady = false;
  });

  $effect(() => {
    editor.protectContent = document?.protectContent ?? false;
  });
  const fontFamilies = $derived(document?.fontFamilies ?? []);

  $effect(() => {
    if (fontFamilies.length > 0) {
      const availableFonts = Object.fromEntries(
        fontFamilies
          .filter((f) => f.state === 'ACTIVE')
          .map((f) => [f.familyName, f.fonts.filter((font) => font.state === 'ACTIVE').map((font) => font.weight)]),
      );
      initWasm().then((wasm) => {
        wasm.setAvailableFonts(availableFonts);
      });
      editor.fontFamilies = fontFamilies;
    }
  });

  const bodySnapshot = $derived(
    document?.documentBody?.__typename === 'DocumentViewBodyAvailable'
      ? Uint8Array.fromBase64(document.documentBody.snapshot)
      : document?.snapshot
        ? Uint8Array.fromBase64(document.snapshot)
        : undefined,
  );
  const assets = $derived(document?.assets);

  $effect(() => {
    if (assets) {
      for (const asset of assets) {
        if (asset.__typename === 'Image') {
          editor.imageAssets.set(asset.id, {
            id: asset.id,
            url: asset.url,
            width: asset.width,
            height: asset.height,
            placeholder: asset.placeholder,
          });
        } else if (asset.__typename === 'File') {
          editor.fileAssets.set(asset.id, {
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
        }
      }
    }
  });

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
</script>

<svelte:head>
  <meta name="robots" content="noindex, nofollow" />
</svelte:head>

{#if document && fontFamilies.length > 0}
  <Helmet
    description={document.excerpt}
    image={{ size: 'large', src: `${env.PUBLIC_API_URL}/og/${entityView.data.id}` }}
    title={document.title}
  />

  {#if document.documentBody.__typename === 'DocumentViewBodyAvailable'}
    {#if bodySnapshot}
      <ReadOnlyTouchSelectionSuppress enabled={editor.touchGesture.gestureActive} />

      {#snippet documentHeader()}
        <div class={css({ paddingTop: { base: '48px', md: '80px' } })}>
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

          {#if editor.layout?.layoutMode.type !== 'paginated'}
            <HorizontalDivider style={css.raw({ marginBottom: '24px' })} />
          {/if}
        </div>
      {/snippet}

      {#if hydrated && !editorReady}
        <div
          style:max-width="640px"
          style:padding-inline="20px"
          class={flex({ flexDirection: 'column', width: 'full', marginX: 'auto' })}
          out:fade={{ duration: 150 }}
        >
          <DocumentViewSkeleton />
        </div>
      {/if}

      {#key document.id}
        {#if document.protectContent}
          <ContentProtect>
            <EditorComponent
              {editor}
              {fontFamilies}
              onEditorReady={() => (editorReady = true)}
              readOnly
              snapshot={bodySnapshot}
              useWindowScroll
            >
              {#snippet header()}
                {@render documentHeader()}
              {/snippet}

              {#snippet footer()}
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

                  <div class={flex({ align: 'center', gap: '12px', marginLeft: 'auto', color: 'text.muted' })}>
                    <ShareLinkPopover href={entityView.data.url} />

                    <DocumentActionMenu entityView$key={entityView.data} />
                  </div>
                </div>

                <div class={css({ paddingBottom: { base: '60px', lg: '80px' } })}>
                  <ContentNavigation entityView$key={entityView.data} />
                </div>
              {/snippet}
            </EditorComponent>
          </ContentProtect>
        {:else}
          <EditorComponent
            {editor}
            {fontFamilies}
            onEditorReady={() => (editorReady = true)}
            readOnly
            snapshot={bodySnapshot}
            useWindowScroll
          >
            {#snippet header()}
              {@render documentHeader()}
            {/snippet}

            {#snippet footer()}
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

                <div class={flex({ align: 'center', marginLeft: 'auto' })}>
                  <ShareLinkPopover href={entityView.data.url} />
                </div>
              </div>

              <div class={css({ paddingBottom: { base: '60px', lg: '80px' } })}>
                <ContentNavigation entityView$key={entityView.data} />
              </div>
            {/snippet}
          </EditorComponent>
        {/if}
      {/key}
    {/if}
  {:else if document.documentBody.__typename === 'DocumentViewBodyUnavailable'}
    <div class={flex({ align: 'center', justify: 'center', minHeight: '[100dvh]', fontSize: '16px', fontWeight: 'medium' })}>
      {#if document.documentBody.reason === 'REQUIRE_IDENTITY_VERIFICATION'}
        <PostViewBodyUnavailable description="본인 인증이 필요한 글이에요" icon={ShieldAlertIcon} title="연령제한글">
          {#if user.data}
            <Button style={css.raw({ width: 'full' })} onclick={handleVerification} variant="secondary">본인 인증</Button>
          {:else}
            <Button style={css.raw({ width: 'full' })} external href={authorizeUrl} type="link" variant="secondary">
              로그인 후 본인 인증하기
            </Button>
          {/if}
        </PostViewBodyUnavailable>
      {:else if document.documentBody.reason === 'REQUIRE_MINIMUM_AGE'}
        <PostViewBodyUnavailable
          description="이 글은 연령 기준에 따라 현재 계정으로는 열람이 제한되어 있어요"
          icon={ShieldAlertIcon}
          title="연령제한글"
        />
      {:else if document.documentBody.reason === 'REQUIRE_PASSWORD'}
        <form onsubmit={form.handleSubmit}>
          <PostViewBodyUnavailable description="해당 내용은 비밀번호 입력이 필요해요" icon={LockIcon} title="비밀글">
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
          </PostViewBodyUnavailable>
        </form>
      {:else}
        {document.documentBody.reason}
      {/if}
    </div>
  {/if}
{/if}
