<script lang="ts">
  import * as PortOne from '@portone/browser-sdk/v2';
  import mixpanel from 'mixpanel-browser';
  import { nanoid } from 'nanoid';
  import qs from 'query-string';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import LockIcon from '~icons/lucide/lock';
  import MessageCircleIcon from '~icons/lucide/message-circle';
  import ShieldAlertIcon from '~icons/lucide/shield-alert';
  import SmileIcon from '~icons/lucide/smile';
  import { page } from '$app/state';
  import { env } from '$env/dynamic/public';
  import { fragment, graphql } from '$graphql';
  import { Button, ContentProtect, Helmet, HorizontalDivider, Icon, Img, TextInput } from '$lib/components';
  import { createForm, FormError } from '$lib/form';
  import { Toast } from '$lib/notification';
  import { TiptapRenderer } from '$lib/tiptap';
  import { comma, serializeOAuthState } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import EmojiReaction from './EmojiReaction.svelte';
  import PostActionMenu from './PostActionMenu.svelte';
  import ShareLinkPopover from './ShareLinkPopover.svelte';
  import type { Optional, UsersiteWildcardSlugPage_PostView_entityView, UsersiteWildcardSlugPage_PostView_user } from '$graphql';

  type Props = {
    $entityView: UsersiteWildcardSlugPage_PostView_entityView;
    $user: Optional<UsersiteWildcardSlugPage_PostView_user>;
  };

  let { $entityView: _entityView, $user: _user }: Props = $props();

  const entityView = fragment(
    _entityView,
    graphql(`
      fragment UsersiteWildcardSlugPage_PostView_entityView on EntityView {
        id
        url

        ancestors {
          id
          url

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

          ... on PostView {
            id
            title
            subtitle
            excerpt
            maxWidth
            protectContent
            allowReaction
            allowComment

            coverImage {
              id
              ...Img_image
            }

            body {
              __typename

              ... on PostViewBodyAvailable {
                content
              }

              ... on PostViewBodyUnavailable {
                reason
              }
            }

            reactions {
              id
              emoji
            }

            comments {
              id
            }

            ...UsersiteWildcardSlugPage_EmojiReaction_postView
          }
        }

        site {
          id

          fonts {
            id
            weight
            url
          }
        }

        ...UsersiteWildcardSlugPage_PostActionMenu_entityView
      }
    `),
  );

  const user = fragment(
    _user,
    graphql(`
      fragment UsersiteWildcardSlugPage_PostView_user on User {
        id
      }
    `),
  );

  const unlockPostView = graphql(`
    mutation UsersiteWildcardSlugPage_UnlockPostView_Mutation($input: UnlockPostViewInput!) {
      unlockPostView(input: $input) {
        id

        body {
          __typename

          ... on PostViewBodyAvailable {
            content
          }

          ... on PostViewBodyUnavailable {
            reason
          }
        }
      }
    }
  `);

  const verifyPersonalIdentity = graphql(`
    mutation UsersiteWildcardSlugPage_VerifyPersonalIdentity_Mutation($input: VerifyPersonalIdentityInput!) {
      verifyPersonalIdentity(input: $input) {
        id

        personalIdentity {
          id
          expiresAt
        }
      }
    }
  `);

  const form = createForm({
    schema: z.object({
      password: z.string(),
    }),
    onSubmit: async (data) => {
      if ($entityView.node.__typename !== 'PostView') {
        return;
      }

      await unlockPostView({
        postId: $entityView.node.id,
        password: data.password,
      });

      mixpanel.track('unlock_post_view', {
        postId: $entityView.node.id,
      });
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'invalid_password') {
        throw new FormError('password', '비밀번호가 올바르지 않습니다.');
      }
    },
  });

  const fontFaces = $derived(
    $entityView.site.fonts
      .map(
        (font) =>
          `@font-face { font-family: ${font.id}; src: url(${font.url}) format('woff2'); font-weight: ${font.weight}; font-display: block; }`,
      )
      .join('\n'),
  );

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
        identityVerificationId: resp.identityVerificationId,
      });

      mixpanel.track('verify_personal_identity_success');
      location.reload();
    } catch (err) {
      const errorMessages: Record<string, string> = {
        identity_verification_failed: '인증에 실패했습니다.',
        same_identity_exists: '이미 다른 계정에 인증된 정보입니다.',
      };

      if (err instanceof TypieError) {
        const message = errorMessages[err.code] || err.code;
        Toast.error(message);
      }
    }
  };
</script>

<svelte:head>
  <meta name="robots" content="noindex, nofollow" />

  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html '<style type="text/css"' + `>${fontFaces}</` + 'style>'}
</svelte:head>

{#if $entityView.node.__typename === 'PostView'}
  <Helmet
    description={$entityView.node.excerpt}
    image={{ size: 'large', src: `${env.PUBLIC_API_URL}/og/${$entityView.id}` }}
    title={$entityView.node.title}
  />

  <div class={flex({ flexDirection: 'column', alignItems: 'center', width: 'full' })}>
    <div
      style:--prosemirror-max-width={`${$entityView.node.maxWidth}px`}
      class={flex({
        flexDirection: 'column',
        alignItems: 'center',
        flexGrow: '1',
        paddingBottom: '80px',
        width: 'full',
        maxWidth: '1200px',
        backgroundColor: 'white',
      })}
    >
      {#if $entityView.node.coverImage}
        <div class={css({ width: 'full', marginBottom: '40px' })}>
          <Img
            style={css.raw({ width: 'full' })}
            $image={$entityView.node.coverImage}
            alt="커버 이미지"
            progressive
            ratio={5 / 2}
            size="full"
          />
        </div>
      {/if}

      <div
        class={css({
          paddingX: '20px',
          paddingTop: $entityView.node.coverImage ? '0' : '50px',
          width: 'full',
          maxWidth: 'var(--prosemirror-max-width)',
        })}
      >
        <div class={flex({ flexDirection: 'column', width: 'full', maxWidth: 'var(--prosemirror-max-width)' })}>
          <div class={flex({ alignItems: 'center', gap: '6px' })}>
            {#each $entityView.ancestors as ancestor (ancestor.id)}
              {#if ancestor.node.__typename === 'FolderView'}
                <a class={css({ fontSize: '14px', color: 'gray.400' })} href={ancestor.url}>{ancestor.node.name}</a>
                <div class={css({ fontSize: '14px', color: 'gray.300' })}>/</div>
              {/if}
            {/each}

            {#if $entityView.ancestors.length > 0}
              <div class={css({ fontSize: '14px' })}>{$entityView.node.title}</div>
            {/if}
          </div>

          <div class={css({ marginTop: '12px', fontSize: '28px', fontWeight: 'bold' })}>
            {$entityView.node.title}
          </div>

          {#if $entityView.node.subtitle}
            <div class={css({ marginTop: '4px', fontSize: '16px', fontWeight: 'medium' })}>
              {$entityView.node.subtitle}
            </div>
          {/if}

          <div class={flex({ align: 'center', justify: 'space-between', marginTop: '20px', paddingBottom: '10px' })}>
            <div class={flex({ align: 'center', gap: '8px', fontSize: '13px', color: 'gray.400' })}>
              {#if $entityView.node.allowReaction && $entityView.node.reactions.length > 0}
                <div class={flex({ align: 'center', gap: '3px' })}>
                  <Icon icon={SmileIcon} />
                  <span class={css({ marginTop: '1px' })}>{comma($entityView.node.reactions.length)}</span>
                </div>
              {/if}

              {#if $entityView.node.allowComment && $entityView.node.comments.length > 0}
                <div class={flex({ align: 'center', gap: '3px' })}>
                  <Icon icon={MessageCircleIcon} />
                  <span class={css({ marginTop: '1px' })}>{comma($entityView.node.comments.length)}</span>
                </div>
              {/if}
            </div>

            <div class={flex({ align: 'center', marginLeft: 'auto', gap: '12px', color: 'gray.600' })}>
              <ShareLinkPopover href={$entityView.url} />

              <PostActionMenu {$entityView} />
            </div>
          </div>

          <HorizontalDivider style={css.raw({ marginBottom: '20px' })} />
        </div>

        {#if $entityView.node.body.__typename === 'PostViewBodyAvailable'}
          {#if $entityView.node.protectContent}
            <ContentProtect>
              <TiptapRenderer style={css.raw({ width: 'full' })} content={$entityView.node.body.content} />
            </ContentProtect>
          {:else}
            <TiptapRenderer style={css.raw({ width: 'full' })} content={$entityView.node.body.content} />
          {/if}

          <div
            class={flex({
              align: 'flex-start',
              justify: 'space-between',
              gap: '8px',
              marginTop: '20px',
              paddingBottom: '10px',
              width: 'full',
              maxWidth: 'var(--prosemirror-max-width)',
            })}
          >
            <EmojiReaction $postView={$entityView.node} />

            <div class={flex({ align: 'center', gap: '12px', marginLeft: 'auto', color: 'gray.600' })}>
              <ShareLinkPopover href={$entityView.url} />

              <PostActionMenu {$entityView} />
            </div>
          </div>
        {:else if $entityView.node.body.__typename === 'PostViewBodyUnavailable'}
          <div class={css({ marginTop: '42px', fontSize: '16px', fontWeight: 'medium' })}>
            {#if $entityView.node.body.reason === 'REQUIRE_IDENTITY_VERIFICATION'}
              <div class={flex({ direction: 'column', align: 'center' })}>
                <div class={center({ borderRadius: '8px', size: '40px', backgroundColor: 'gray.100', color: 'gray.700' })}>
                  <Icon icon={ShieldAlertIcon} size={20} />
                </div>
                <p class={css({ marginTop: '10px', marginBottom: '2px', fontSize: '18px', fontWeight: 'semibold' })}>연령제한글</p>
                <p class={css({ marginBottom: '20px', fontSize: '14px', color: 'gray.600' })}>본인 인증이 필요한 글이에요</p>

                {#if $user}
                  <Button
                    style={css.raw({ width: 'full', maxWidth: '280px', height: '38px', borderRadius: '6px' })}
                    onclick={handleVerification}
                  >
                    본인 인증
                  </Button>
                {:else}
                  <Button
                    style={css.raw({ width: 'full', maxWidth: '280px', height: '38px', borderRadius: '6px' })}
                    external
                    href={authorizeUrl}
                    type="link"
                  >
                    로그인 후 본인 인증하기
                  </Button>
                {/if}
              </div>
            {:else if $entityView.node.body.reason === 'REQUIRE_MINIMUM_AGE'}
              <div class={flex({ direction: 'column', align: 'center' })}>
                <div class={center({ borderRadius: '8px', size: '40px', backgroundColor: 'gray.100', color: 'gray.700' })}>
                  <Icon icon={ShieldAlertIcon} size={20} />
                </div>

                <p class={css({ marginTop: '10px', marginBottom: '2px', fontSize: '18px', fontWeight: 'semibold' })}>연령제한글</p>
                <p class={css({ marginBottom: '20px', fontSize: '14px', color: 'gray.600', textAlign: 'center' })}>
                  이 글은 연령 기준에 따라 현재 계정으로는 열람이 제한되어 있어요
                </p>
              </div>
            {:else if $entityView.node.body.reason === 'REQUIRE_PASSWORD'}
              <form class={flex({ direction: 'column', align: 'center' })} onsubmit={form.handleSubmit}>
                <div class={center({ borderRadius: '8px', size: '40px', backgroundColor: 'gray.100', color: 'gray.700' })}>
                  <Icon icon={LockIcon} size={20} />
                </div>
                <p class={css({ marginTop: '10px', marginBottom: '2px', fontSize: '18px', fontWeight: 'semibold' })}>비밀글</p>
                <p class={css({ marginBottom: '20px', fontSize: '14px', color: 'gray.600' })}>해당 내용은 비밀번호 입력이 필요해요</p>

                <div class={flex({ direction: 'column', align: 'center', gap: '12px', width: 'full', maxWidth: '280px' })}>
                  <TextInput
                    id="password"
                    style={css.raw({ width: 'full' })}
                    placeholder="비밀번호를 입력하세요"
                    bind:value={form.fields.password}
                  />
                  {#if form.errors.password}
                    <p class={css({ color: 'red.500', fontSize: '14px' })}>{form.errors.password}</p>
                  {/if}

                  <Button style={css.raw({ width: 'full', height: '38px', borderRadius: '6px' })} size="lg" type="submit">확인</Button>
                </div>
              </form>
            {:else}
              {$entityView.node.body.reason}
            {/if}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
