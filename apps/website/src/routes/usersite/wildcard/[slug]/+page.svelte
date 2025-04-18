<script lang="ts">
  import { onMount } from 'svelte';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import LockKeyholeIcon from '~icons/lucide/lock-keyhole';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { Button, Helmet, HorizontalDivider, Icon, Img, ProtectiveRegion, TextInput } from '$lib/components';
  import { createForm, FormError } from '$lib/form';
  import { TiptapRenderer } from '$lib/tiptap';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import Header from './Header.svelte';

  const query = graphql(`
    query UsersiteWildcardSlugPage_Query($origin: String!, $slug: String!) {
      entityView(origin: $origin, slug: $slug) {
        id

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
            }

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
          }

          ... on FolderView {
            id
            name
          }
        }
      }
    }
  `);

  const clientQuery = graphql(`
    query UsersiteWildcardSlugPage_Client_Query @client {
      me {
        id

        ...UsersiteWildcardSlugPage_Header_user
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

  let loading = $state(true);

  const load = async () => {
    try {
      await clientQuery.load();
    } finally {
      loading = false;
    }
  };

  onMount(() => {
    load();
  });
</script>

{#if $query.entityView.node.__typename === 'PostView'}
  <Helmet
    description={$query.entityView.node.excerpt}
    image={{ size: 'large', src: `${env.PUBLIC_API_URL}/og/${$query.entityView.id}` }}
    title={$query.entityView.node.title}
  />

  <Header $user={$clientQuery?.me ?? null} {loading} />

  <div class={flex({ flexDirection: 'column', alignItems: 'center', width: 'full', minHeight: 'screen', backgroundColor: 'gray.100' })}>
    <div
      style:--prosemirror-max-width={`${$query.entityView.node.maxWidth}px`}
      class={flex({
        flexDirection: 'column',
        alignItems: 'center',
        flexGrow: '1',
        paddingX: '20px',
        paddingY: '80px',
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
        <div class={css({ fontSize: '28px', fontWeight: 'bold' })}>
          {$query.entityView.node.title}
        </div>

        {#if $query.entityView.node.subtitle}
          <div class={css({ marginTop: '4px', fontSize: '16px', fontWeight: 'medium' })}>
            {$query.entityView.node.subtitle}
          </div>
        {/if}

        <HorizontalDivider style={css.raw({ marginTop: '10px', marginBottom: '20px' })} />
      </div>

      {#if $query.entityView.node.body.__typename === 'PostViewBodyAvailable'}
        {#if $query.entityView.node.option.protectContent}
          <TiptapRenderer style={css.raw({ width: 'full' })} content={$query.entityView.node.body.content} />
        {:else}
          <ProtectiveRegion>
            <TiptapRenderer style={css.raw({ width: 'full' })} content={$query.entityView.node.body.content} />
          </ProtectiveRegion>
        {/if}
      {:else if $query.entityView.node.body.__typename === 'PostViewBodyUnavailable'}
        <div class={css({ marginTop: '42px', fontSize: '16px', fontWeight: 'medium' })}>
          {#if $query.entityView.node.body.reason === 'REQUIRE_IDENTITY_VERIFICATION'}
            <div class={flex({ direction: 'column', align: 'center', gap: '16px' })}>
              {#if loading}
                <div
                  class={css({
                    marginTop: '12px',
                    borderRadius: '4px',
                    backgroundColor: 'gray.100',
                    width: '190px',
                    height: '22px',
                  })}
                ></div>

                <div class={css({ borderRadius: '8px', backgroundColor: 'gray.100', width: '92px', height: '36px' })}></div>
              {:else}
                <p class={css({ marginTop: '12px' })}>본인 인증이 필요한 글이에요.</p>

                {#if $clientQuery?.me}
                  <!-- TODO: 설정 모달 바로 띄우기 -->
                  <Button style={css.raw({ width: '92px' })} external href={`${env.PUBLIC_WEBSITE_URL}/home`} type="link">본인 인증</Button>
                {:else}
                  <p>로그인 후 본인인증을 진행해 주세요.</p>

                  <Button external href={`${env.PUBLIC_WEBSITE_URL}/auth/login`} type="link">로그인 후 본인 인증하기</Button>
                {/if}
              {/if}
            </div>
          {:else if $query.entityView.node.body.reason === 'REQUIRE_MINIMUM_AGE'}
            <p>이 글은 연령 기준에 따라 현재 계정으로는 열람이 제한되어 있어요.</p>
          {:else if $query.entityView.node.body.reason === 'REQUIRE_PASSWORD'}
            <form class={flex({ direction: 'column', align: 'center' })} onsubmit={form.handleSubmit}>
              <Icon icon={LockKeyholeIcon} size={32} />
              <p class={css({ marginTop: '12px', marginBottom: '16px' })}>해당 내용은 비밀번호 입력이 필요해요</p>

              <div class={flex({ direction: 'column', align: 'center', gap: '8px' })}>
                <TextInput
                  id="password"
                  style={css.raw({ width: '280px' })}
                  placeholder="비밀번호를 입력하세요"
                  bind:value={form.fields.password}
                />
                {#if form.errors.password}
                  <p class={css({ color: 'red.500', fontSize: '14px' })}>{form.errors.password}</p>
                {/if}

                <Button style={css.raw({ width: '92px' })} type="submit">확인</Button>
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
