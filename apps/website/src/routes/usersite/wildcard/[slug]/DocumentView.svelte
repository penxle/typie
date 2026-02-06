<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, ContentProtect, Helmet, HorizontalDivider, Icon, TextInput } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import { comma } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import LockIcon from '~icons/lucide/lock';
  import ShieldAlertIcon from '~icons/lucide/shield-alert';
  import SmileIcon from '~icons/lucide/smile';
  import { env } from '$env/dynamic/public';
  import { fragment, graphql } from '$graphql';
  import { Editor as EditorComponent } from '$lib/components/editor';
  import { setEditor } from '$lib/editor/context';
  import { Editor } from '$lib/editor/editor.svelte';
  import PostViewBodyUnavailable from './PostViewBodyUnavailable.svelte';
  import ShareLinkPopover from './ShareLinkPopover.svelte';
  import type { UsersiteWildcardSlugPage_DocumentView_entityView } from '$graphql';

  type Props = {
    $entityView: UsersiteWildcardSlugPage_DocumentView_entityView;
  };

  let { $entityView: _entityView }: Props = $props();

  const entityView = fragment(
    _entityView,
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
          }
        }

        site {
          id

          fonts {
            id
            weight
            url

            family {
              id
            }
          }
        }
      }
    `),
  );

  const unlockDocumentView = graphql(`
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
  `);

  const form = createForm({
    schema: z.object({
      password: z.string(),
    }),
    onSubmit: async (data) => {
      if ($entityView.node.__typename !== 'DocumentView') {
        return;
      }

      await unlockDocumentView({
        documentId: $entityView.node.id,
        password: data.password,
      });

      mixpanel.track('unlock_document_view', {
        documentId: $entityView.node.id,
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

  const editor = new Editor();
  setEditor(editor);

  const document = $derived($entityView.node.__typename === 'DocumentView' ? $entityView.node : null);

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

  const fontFaces = $derived(
    $entityView.site.fonts
      .flatMap((font) => [
        `@font-face { font-family: ${font.id}; src: url(${font.url}) format('woff2'); font-weight: ${font.weight}; font-display: block; }`,
        `@font-face { font-family: ${font.family.id}; src: url(${font.url}) format('woff2'); font-weight: ${font.weight}; font-display: block; }`,
      ])
      .join('\n'),
  );
</script>

<svelte:head>
  <meta name="robots" content="noindex, nofollow" />

  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html '<style type="text/css"' + `>${fontFaces}</` + 'style>'}
</svelte:head>

{#if document}
  <Helmet
    description={document.excerpt}
    image={{ size: 'large', src: `${env.PUBLIC_API_URL}/og/${$entityView.id}` }}
    title={document.title}
  />

  {#if document.documentBody.__typename === 'DocumentViewBodyAvailable'}
    {#if bodySnapshot}
      {#if document.protectContent}
        <ContentProtect>
          <EditorComponent {editor} readOnly snapshot={bodySnapshot} useWindowScroll>
            {#snippet header()}
              <div class={css({ paddingTop: { base: '24px', md: '48px' } })}>
                <div class={flex({ alignItems: 'center', gap: '4px', wrap: 'wrap', marginBottom: { base: '4px', lg: '8px' } })}>
                  {#each $entityView.ancestors as ancestor (ancestor.id)}
                    {#if ancestor.node.__typename === 'FolderView'}
                      <a class={css({ fontSize: { base: '12px', lg: '13px' }, color: 'text.disabled' })} href={`/${ancestor.slug}`}>
                        {ancestor.node.name}
                      </a>
                      <div class={css({ fontSize: { base: '12px', lg: '13px' }, color: 'text.disabled' })}>/</div>
                    {/if}
                  {/each}

                  {#if $entityView.ancestors.length > 0}
                    <div class={css({ fontSize: { base: '12px', lg: '13px' }, color: 'text.subtle' })}>{document.title}</div>
                  {/if}
                </div>

                <div class={css({ fontSize: { base: '24px', lg: '28px' }, fontWeight: 'bold' })}>
                  {document.title}
                </div>

                {#if document.subtitle}
                  <div class={css({ marginTop: '4px', fontSize: { base: '14px', lg: '16px' }, fontWeight: 'medium' })}>
                    {document.subtitle}
                  </div>
                {/if}

                <div class={flex({ align: 'center', justify: 'space-between', marginTop: '20px', paddingBottom: '10px' })}>
                  <div class={flex({ align: 'center', gap: '8px', fontSize: '13px', color: 'text.faint' })}>
                    {#if document.allowReaction && document.reactions.length > 0}
                      <div class={flex({ align: 'center', gap: '3px' })}>
                        <Icon icon={SmileIcon} />
                        <span>{comma(document.reactions.length)}</span>
                      </div>
                    {/if}
                  </div>

                  <div class={flex({ align: 'center', marginLeft: 'auto' })}>
                    <ShareLinkPopover href={$entityView.url} />
                  </div>
                </div>

                <HorizontalDivider style={css.raw({ marginBottom: '20px' })} />
              </div>
            {/snippet}
          </EditorComponent>
        </ContentProtect>
      {:else}
        <EditorComponent {editor} readOnly snapshot={bodySnapshot} useWindowScroll>
          {#snippet header()}
            <div class={css({ paddingTop: { base: '24px', md: '48px' } })}>
              <div class={flex({ alignItems: 'center', gap: '4px', wrap: 'wrap', marginBottom: { base: '4px', lg: '8px' } })}>
                {#each $entityView.ancestors as ancestor (ancestor.id)}
                  {#if ancestor.node.__typename === 'FolderView'}
                    <a class={css({ fontSize: { base: '12px', lg: '13px' }, color: 'text.disabled' })} href={`/${ancestor.slug}`}>
                      {ancestor.node.name}
                    </a>
                    <div class={css({ fontSize: { base: '12px', lg: '13px' }, color: 'text.disabled' })}>/</div>
                  {/if}
                {/each}

                {#if $entityView.ancestors.length > 0}
                  <div class={css({ fontSize: { base: '12px', lg: '13px' }, color: 'text.subtle' })}>{document.title}</div>
                {/if}
              </div>

              <div class={css({ fontSize: { base: '24px', lg: '28px' }, fontWeight: 'bold' })}>
                {document.title}
              </div>

              {#if document.subtitle}
                <div class={css({ marginTop: '4px', fontSize: { base: '14px', lg: '16px' }, fontWeight: 'medium' })}>
                  {document.subtitle}
                </div>
              {/if}

              <div class={flex({ align: 'center', justify: 'space-between', marginTop: '20px', paddingBottom: '10px' })}>
                <div class={flex({ align: 'center', gap: '8px', fontSize: '13px', color: 'text.faint' })}>
                  {#if document.allowReaction && document.reactions.length > 0}
                    <div class={flex({ align: 'center', gap: '3px' })}>
                      <Icon icon={SmileIcon} />
                      <span>{comma(document.reactions.length)}</span>
                    </div>
                  {/if}
                </div>

                <div class={flex({ align: 'center', marginLeft: 'auto' })}>
                  <ShareLinkPopover href={$entityView.url} />
                </div>
              </div>

              <HorizontalDivider style={css.raw({ marginBottom: '20px' })} />
            </div>
          {/snippet}
        </EditorComponent>
      {/if}
    {/if}
  {:else if document.documentBody.__typename === 'DocumentViewBodyUnavailable'}
    <div class={css({ marginTop: '42px', fontSize: '16px', fontWeight: 'medium' })}>
      {#if document.documentBody.reason === 'REQUIRE_IDENTITY_VERIFICATION'}
        <PostViewBodyUnavailable description="본인 인증이 필요한 글이에요" icon={ShieldAlertIcon} title="연령제한글" />
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
