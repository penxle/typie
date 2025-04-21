<script lang="ts">
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import EllipsisVerticalIcon from '~icons/lucide/ellipsis-vertical';
  import LockIcon from '~icons/lucide/lock';
  import MessageCircleIcon from '~icons/lucide/message-circle';
  import ShieldAlertIcon from '~icons/lucide/shield-alert';
  import SmileIcon from '~icons/lucide/smile';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { Button, ContentProtect, Helmet, HorizontalDivider, Icon, Img, TextInput } from '$lib/components';
  import { createForm, FormError } from '$lib/form';
  import { TiptapRenderer } from '$lib/tiptap';
  import { comma } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import Header from './Header.svelte';
  import ShareLinkPopover from './ShareLinkPopover.svelte';

  const query = graphql(`
    query UsersiteWildcardSlugPage_Query($origin: String!, $slug: String!) {
      me {
        id

        ...UsersiteWildcardSlugPage_Header_user
      }

      entityView(origin: $origin, slug: $slug) {
        id

        ancestors {
          id

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

            option {
              id
              protectContent
              allowReaction
              allowComment
            }

            coverImage {
              id
              ...Img_image
            }

            entity {
              id
              url
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
            }

            comments {
              id
            }
          }

          ... on FolderView {
            id
            name
          }
        }
      }
    }
  `);

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

  const form = createForm({
    schema: z.object({
      password: z.string(),
    }),
    onSubmit: async (data) => {
      await unlockPostView({
        postId: $query.entityView.node.id,
        password: data.password,
      });
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'invalid_password') {
        throw new FormError('password', '비밀번호가 올바르지 않습니다.');
      }
    },
  });
</script>

<svelte:head>
  <meta name="robots" content="noindex, nofollow" />
</svelte:head>

{#if $query.entityView.node.__typename === 'PostView'}
  <Helmet
    description={$query.entityView.node.excerpt}
    image={{ size: 'large', src: `${env.PUBLIC_API_URL}/og/${$query.entityView.id}` }}
    title={$query.entityView.node.title}
  />

  <Header $user={$query.me} />

  <div class={flex({ flexDirection: 'column', alignItems: 'center', width: 'full', minHeight: 'screen' })}>
    <div
      style:--prosemirror-max-width={`${$query.entityView.node.maxWidth}px`}
      class={flex({
        flexDirection: 'column',
        alignItems: 'center',
        flexGrow: '1',
        paddingX: '20px',
        paddingTop: '50px',
        paddingBottom: '80px',
        width: 'full',
        maxWidth: '1200px',
        backgroundColor: 'white',
      })}
    >
      {#if $query.entityView.node.coverImage}
        <div class={css({ width: 'full', marginBottom: '40px' })}>
          <Img
            style={css.raw({ width: 'full' })}
            $image={$query.entityView.node.coverImage}
            alt="커버 이미지"
            progressive
            ratio={5 / 2}
            size="full"
          />
        </div>
      {/if}

      <div class={flex({ flexDirection: 'column', width: 'full', maxWidth: 'var(--prosemirror-max-width)' })}>
        <div class={flex({ alignItems: 'center', gap: '6px' })}>
          {#each $query.entityView.ancestors as ancestor (ancestor.id)}
            {#if ancestor.node.__typename === 'FolderView'}
              <div class={css({ fontSize: '14px', color: 'gray.400' })}>{ancestor.node.name}</div>
              <div class={css({ fontSize: '14px', color: 'gray.300' })}>/</div>
            {/if}
          {/each}

          {#if $query.entityView.ancestors.length > 0}
            <div class={css({ fontSize: '14px' })}>{$query.entityView.node.title}</div>
          {/if}
        </div>

        <div class={css({ marginTop: '12px', fontSize: '28px', fontWeight: 'bold' })}>
          {$query.entityView.node.title}
        </div>

        {#if $query.entityView.node.subtitle}
          <div class={css({ marginTop: '4px', fontSize: '16px', fontWeight: 'medium' })}>
            {$query.entityView.node.subtitle}
          </div>
        {/if}

        <div class={flex({ align: 'center', justify: 'space-between', marginTop: '20px', paddingBottom: '10px' })}>
          <div class={flex({ align: 'center', gap: '8px', fontSize: '13px', color: 'gray.400' })}>
            {#if $query.entityView.node.option.allowReaction && $query.entityView.node.reactions.length > 0}
              <div class={flex({ align: 'center', gap: '3px' })}>
                <Icon icon={SmileIcon} />
                <span class={css({ marginTop: '1px' })}>{comma($query.entityView.node.reactions.length)}</span>
              </div>
            {/if}

            {#if $query.entityView.node.option.allowComment && $query.entityView.node.comments.length > 0}
              <div class={flex({ align: 'center', gap: '3px' })}>
                <Icon icon={MessageCircleIcon} />
                <span class={css({ marginTop: '1px' })}>{comma($query.entityView.node.comments.length)}</span>
              </div>
            {/if}
          </div>

          <div class={flex({ align: 'center', marginLeft: 'auto', gap: '16px', color: 'gray.600' })}>
            <ShareLinkPopover href={$query.entityView.node.entity.url} />

            <button type="button">
              <Icon icon={EllipsisVerticalIcon} size={18} />
            </button>
          </div>
        </div>

        <HorizontalDivider style={css.raw({ marginBottom: '20px' })} />
      </div>

      {#if $query.entityView.node.body.__typename === 'PostViewBodyAvailable'}
        {#if $query.entityView.node.option.protectContent}
          <ContentProtect>
            <TiptapRenderer style={css.raw({ width: 'full' })} content={$query.entityView.node.body.content} />
          </ContentProtect>
        {:else}
          <TiptapRenderer style={css.raw({ width: 'full' })} content={$query.entityView.node.body.content} />
        {/if}

        <div
          class={flex({
            align: 'center',
            justify: 'space-between',
            marginTop: '20px',
            paddingBottom: '10px',
            width: 'full',
            maxWidth: 'var(--prosemirror-max-width)',
          })}
        >
          <!-- TODO: 이모지 -->

          <div class={flex({ align: 'center', gap: '16px', marginLeft: 'auto', color: 'gray.600' })}>
            <ShareLinkPopover href={$query.entityView.node.entity.url} />

            <button type="button">
              <Icon icon={EllipsisVerticalIcon} size={18} />
            </button>
          </div>
        </div>
      {:else if $query.entityView.node.body.__typename === 'PostViewBodyUnavailable'}
        <div class={css({ marginTop: '42px', fontSize: '16px', fontWeight: 'medium' })}>
          {#if $query.entityView.node.body.reason === 'REQUIRE_IDENTITY_VERIFICATION'}
            <div class={flex({ direction: 'column', align: 'center' })}>
              <div class={center({ borderRadius: 'full', size: '48px', backgroundColor: 'gray.100', color: 'gray.700' })}>
                <Icon icon={ShieldAlertIcon} size={24} />
              </div>
              <p class={css({ marginTop: '12px', marginBottom: '8px', fontSize: '20px', fontWeight: 'semibold' })}>연령제한글</p>
              <p class={css({ marginBottom: '20px', fontSize: '14px', color: 'gray.600' })}>본인 인증이 필요한 글이에요</p>

              {#if $query.me}
                <Button style={css.raw({ width: 'full', height: '44px' })} external href={`${env.PUBLIC_WEBSITE_URL}/home`} type="link">
                  본인 인증
                </Button>
              {:else}
                <Button
                  style={css.raw({ width: 'full', height: '44px' })}
                  external
                  href={`${env.PUBLIC_WEBSITE_URL}/auth/login`}
                  type="link"
                >
                  로그인 후 본인 인증하기
                </Button>
              {/if}
            </div>
          {:else if $query.entityView.node.body.reason === 'REQUIRE_MINIMUM_AGE'}
            <div class={flex({ direction: 'column', align: 'center' })}>
              <div class={center({ borderRadius: 'full', size: '48px', backgroundColor: 'gray.100', color: 'gray.700' })}>
                <Icon icon={ShieldAlertIcon} size={24} />
              </div>
              <p class={css({ marginTop: '12px', marginBottom: '8px', fontSize: '20px', fontWeight: 'semibold' })}>연령제한글</p>
              <p class={css({ marginBottom: '20px', fontSize: '14px', color: 'gray.600' })}>
                이 글은 연령 기준에 따라 현재 계정으로는 열람이 제한되어 있어요
              </p>
            </div>
          {:else if $query.entityView.node.body.reason === 'REQUIRE_PASSWORD'}
            <form class={flex({ direction: 'column', align: 'center' })} onsubmit={form.handleSubmit}>
              <div class={center({ borderRadius: 'full', size: '48px', backgroundColor: 'gray.100', color: 'gray.700' })}>
                <Icon icon={LockIcon} size={24} />
              </div>
              <p class={css({ marginTop: '12px', marginBottom: '8px', fontSize: '20px', fontWeight: 'semibold' })}>비밀글</p>
              <p class={css({ marginBottom: '20px', fontSize: '14px', color: 'gray.600' })}>해당 내용은 비밀번호 입력이 필요해요</p>

              <div class={flex({ direction: 'column', align: 'center', gap: '12px' })}>
                <TextInput
                  id="password"
                  style={css.raw({ width: '280px' })}
                  placeholder="비밀번호를 입력하세요"
                  bind:value={form.fields.password}
                />
                {#if form.errors.password}
                  <p class={css({ color: 'red.500', fontSize: '14px' })}>{form.errors.password}</p>
                {/if}

                <Button style={css.raw({ width: 'full', height: '44px' })} size="lg" type="submit">확인</Button>
              </div>
            </form>
          {:else}
            {$query.entityView.node.body.reason}
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}
