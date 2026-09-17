<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import * as PortOne from '@portone/browser-sdk/v2';
  import { TypieError } from '@typie/lib/errors';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, ContentProtect, Helmet, TextInput } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import { createForm, FormError } from '@typie/ui/form';
  import { Toast } from '@typie/ui/notification';
  import { serializeOAuthState } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import qs from 'query-string';
  import { onDestroy, untrack } from 'svelte';
  import { z } from 'zod';
  import LockIcon from '~icons/lucide/lock';
  import ShieldAlertIcon from '~icons/lucide/shield-alert';
  import { page } from '$app/state';
  import { env } from '$env/dynamic/public';
  import { Editor as EditorComponent, EditorFailureOverlay } from '$lib/editor-ffi/components';
  import { Editor, setupEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { registerLinkContextMenu } from '$lib/editor-ffi/handlers/link';
  import { browserScaleFactor } from '$lib/editor-ffi/zoom';
  import { unwrapError } from '$lib/graphql';
  import { graphql } from '$mearie';
  import BodyUnavailable from './BodyUnavailable.svelte';
  import { parseDocumentViewLayoutMode } from './document-view-layout';
  import DocumentDomMirror from './DocumentDomMirror.svelte';
  import DocumentViewFrame from './DocumentViewFrame.svelte';
  import type { Snippet } from 'svelte';
  import type { UsersiteReadingView_document$key, UsersiteReadingView_user$key } from '$mearie';

  type Props = {
    document$key: UsersiteReadingView_document$key;
    user$key: UsersiteReadingView_user$key | null | undefined;
    ogImageUrl: string;
    helmetTrailing?: string;
    unlock: (password: string) => Promise<void>;
    head: Snippet;
    foot: Snippet;
  };

  type EditorFailure = {
    documentId: string;
    editor?: Editor;
  };

  let { document$key, user$key, ogImageUrl, helmetTrailing, unlock, head, foot }: Props = $props();

  const fragment = createFragment(
    graphql(`
      fragment UsersiteReadingView_document on IEditorDocument {
        __typename
        id
        ...Editor_document

        ... on DocumentView {
          id
          title
          subtitle
          excerpt
          hasPassword
          protectContent
          layoutMode

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

        ... on PublicationView {
          id
          documentId
          title
          subtitle
          excerpt
          hasPassword
          protectContent
          layoutMode

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
    () => document$key,
  );

  const user = createFragment(
    graphql(`
      fragment UsersiteReadingView_user on User {
        id
      }
    `),
    () => user$key,
  );

  const [verifyPersonalIdentity] = createMutation(
    graphql(`
      mutation UsersiteReadingView_VerifyPersonalIdentity_Mutation($input: VerifyPersonalIdentityInput!) {
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

  const document = $derived(fragment.data.__typename === 'Document' ? null : fragment.data);
  const documentId = $derived(document?.__typename === 'PublicationView' ? document.documentId : document?.id);

  const form = createForm({
    schema: z.object({
      password: z.string().trim(),
    }),
    onSubmit: async (data) => {
      if (!documentId) return;

      await unlock(data.password);

      mixpanel.track('unlock_document_view', { documentId });
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

  const editorKey = $derived(document?.id);
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
        editor.nativeSelection = true;
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
      } else if (asset.__typename === 'DocumentArchivedNode') {
        editor.archivedAssets.set(asset.id, {
          id: asset.id,
          content: asset.content,
        });
      }
    }
  });

  const layoutMode = $derived(parseDocumentViewLayoutMode(document?.layoutMode));
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
  <Helmet description={document.excerpt} image={{ size: 'large', src: ogImageUrl }} title={document.title} trailing={helmetTrailing} />

  {#if document.documentBody.__typename === 'DocumentViewBodyAvailableV2'}
    {#if graph}
      {#key document.id}
        <div class={css({ position: 'relative', isolation: 'isolate' })}>
          <DocumentViewFrame {layoutMode} ready={editorReady} bind:bodySurface={fallbackBodySurface}>
            {#snippet header()}
              {@render head()}
            {/snippet}

            {#snippet footer()}
              {@render foot()}
            {/snippet}

            {#if document.protectContent}
              <ContentProtect>
                <EditorComponent
                  style={css.raw({ paddingBottom: isPaginated ? '40px' : '0' })}
                  active={false}
                  document$key={document}
                  mode="viewer"
                  onReady={handleEditorReady}
                  useWindowScroll
                />
              </ContentProtect>
            {:else}
              <EditorComponent
                style={css.raw({ paddingBottom: isPaginated ? '40px' : '0' })}
                active={false}
                document$key={document}
                mode="viewer"
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
