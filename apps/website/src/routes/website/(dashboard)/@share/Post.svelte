<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon, Select, Switch } from '@typie/ui/components';
  import { createForm } from '@typie/ui/form';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { EntityAvailability, EntityVisibility, PostContentRating } from '@/enums';
  import BanIcon from '~icons/lucide/ban';
  import BlendIcon from '~icons/lucide/blend';
  import CheckIcon from '~icons/lucide/check';
  import EyeIcon from '~icons/lucide/eye';
  import EyeOffIcon from '~icons/lucide/eye-off';
  import IdCardIcon from '~icons/lucide/id-card';
  import LinkIcon from '~icons/lucide/link';
  import LockIcon from '~icons/lucide/lock';
  import LockKeyholeIcon from '~icons/lucide/lock-keyhole';
  import ShieldIcon from '~icons/lucide/shield';
  import SmileIcon from '~icons/lucide/smile';
  import UsersRoundIcon from '~icons/lucide/users-round';
  import { env } from '$env/dynamic/public';
  import { fragment, graphql } from '$graphql';
  import type { DashboardLayout_Share_Post_post } from '$graphql';

  type Props = {
    $post: DashboardLayout_Share_Post_post;
  };

  let { $post: _post }: Props = $props();

  let activeTab = $state<'publish' | 'share'>('publish');

  const post = fragment(
    _post,
    graphql(`
      fragment DashboardLayout_Share_Post_post on Post {
        id
        title
        password
        contentRating
        allowComment
        allowReaction
        protectContent

        entity {
          id
          url
          slug
          visibility
          availability
        }
      }
    `),
  );

  const updatePostOption = graphql(`
    mutation DashboardLayout_Share_Post_UpdatePostOption_Mutation($input: UpdatePostOptionInput!) {
      updatePostOption(input: $input) {
        id
        password
        contentRating
        allowComment
        allowReaction
        protectContent

        entity {
          id
          visibility
          availability
        }
      }
    }
  `);

  let copied = $state(false);
  let timer: NodeJS.Timeout | undefined;

  let showPassword = $state(false);

  const form = createForm({
    schema: z.object({
      availability: z.nativeEnum(EntityAvailability),
      visibility: z.nativeEnum(EntityVisibility),
      hasPassword: z.boolean(),
      password: z.string().nullish(),
      contentRating: z.nativeEnum(PostContentRating),
      allowComment: z.boolean(),
      allowReaction: z.boolean(),
      protectContent: z.boolean(),
    }),
    submitOn: 'change',
    onSubmit: async (data) => {
      await updatePostOption({
        postId: $post.id,
        availability: data.availability,
        visibility: data.visibility,
        password: data.hasPassword ? data.password : null,
        contentRating: data.contentRating,
        allowComment: data.allowComment,
        allowReaction: data.allowReaction,
        protectContent: data.protectContent,
      });

      mixpanel.track('update_post_option', {
        availability: data.availability,
        visibility: data.visibility,
        hasPassword: data.hasPassword,
        contentRating: data.contentRating,
        allowComment: data.allowComment,
        allowReaction: data.allowReaction,
        protectContent: data.protectContent,
      });
    },
    defaultValues: {
      availability: $post.entity.availability,
      visibility: $post.entity.visibility,
      hasPassword: $post.password !== null,
      password: $post.password,
      contentRating: $post.contentRating,
      allowComment: $post.allowComment,
      allowReaction: $post.allowReaction,
      protectContent: $post.protectContent,
    },
  });

  $effect(() => {
    void form;
  });

  const handleCopyLink = () => {
    navigator.clipboard.writeText(activeTab === 'publish' ? $post.entity.url : `${env.PUBLIC_WEBSITE_URL}/${$post.entity.slug}`);
    mixpanel.track('copy_post_share_url', { tab: activeTab });

    if (timer) {
      clearTimeout(timer);
    }

    copied = true;
    timer = setTimeout(() => (copied = false), 2000);
  };
</script>

<div class={css({ position: 'relative' })}>
  <div class={flex({ alignItems: 'center', paddingX: '16px' })}>
    <button
      class={css({
        position: 'relative',
        padding: '12px',
        fontSize: '12px',
        fontWeight: 'medium',
        color: activeTab === 'publish' ? 'text.default' : 'text.muted',
        transition: 'common',
        _hover: {
          color: activeTab === 'publish' ? 'text.default' : 'text.subtle',
        },
        _after: {
          content: '""',
          position: 'absolute',
          bottom: '0',
          insetX: '0',
          height: '3px',
          backgroundColor: activeTab === 'publish' ? 'accent.brand.default' : 'transparent',
          transition: 'common',
        },
      })}
      onclick={() => (activeTab = 'publish')}
      type="button"
    >
      조회
    </button>
    <button
      class={css({
        position: 'relative',
        padding: '12px',
        fontSize: '12px',
        fontWeight: 'medium',
        color: activeTab === 'share' ? 'text.default' : 'text.muted',
        transition: 'common',
        _hover: {
          color: activeTab === 'share' ? 'text.default' : 'text.subtle',
        },
        _after: {
          content: '""',
          position: 'absolute',
          bottom: '0',
          insetX: '0',
          height: '3px',
          backgroundColor: activeTab === 'share' ? 'accent.brand.default' : 'transparent',
          transition: 'common',
        },
      })}
      onclick={() => (activeTab = 'share')}
      type="button"
    >
      편집
    </button>

    <div class={flex({ flexGrow: '1' })}></div>

    <button
      class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}
      onclick={handleCopyLink}
      type="button"
      use:tooltip={{
        message:
          activeTab === 'publish'
            ? form.fields.visibility === EntityVisibility.PRIVATE
              ? '지금은 링크가 있어도 나만 볼 수 있어요'
              : '링크가 있는 누구나 포스트를 볼 수 있어요'
            : form.fields.availability === EntityAvailability.PRIVATE
              ? '지금은 링크가 있어도 나만 편집할 수 있어요'
              : '링크가 있는 누구나 편집할 수 있어요',
        placement: 'top',
        keepOnClick: true,
      }}
    >
      {#if copied}
        <Icon style={css.raw({ color: 'text.link' })} icon={CheckIcon} size={12} />
        <div class={css({ fontSize: '12px', color: 'text.link' })}>복사되었어요</div>
      {:else}
        <Icon style={css.raw({ color: 'text.link' })} icon={LinkIcon} size={12} />
        <div class={css({ fontSize: '12px', color: 'text.link' })}>{activeTab === 'publish' ? '조회' : '편집'} 링크 복사</div>
      {/if}
    </button>
  </div>

  <div
    class={css({
      position: 'absolute',
      insetX: '0',
      bottom: '0',
      height: '1px',
      backgroundColor: 'border.default',
    })}
  ></div>
</div>

{#if activeTab === 'publish'}
  <div class={flex({ flexDirection: 'column', gap: '16px', paddingX: '16px', paddingTop: '16px', paddingBottom: '24px' })}>
    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>포스트 조회 권한</div>

      <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={BlendIcon} size={14} />
          <div class={css({ fontSize: '12px', color: 'text.subtle' })}>공개 범위</div>
        </div>

        <Select
          items={[
            {
              icon: LinkIcon,
              label: '링크가 있는 사람',
              description: '링크가 있는 누구나 볼 수 있어요.',
              value: EntityVisibility.UNLISTED,
            },
            {
              icon: LockIcon,
              label: '비공개',
              description: '나만 볼 수 있어요.',
              value: EntityVisibility.PRIVATE,
            },
          ]}
          bind:value={form.fields.visibility}
        />
      </div>

      <div class={flex({ flexDirection: 'column', gap: '8px' })}>
        <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={LockKeyholeIcon} />
            <div class={css({ fontSize: '12px', color: 'text.subtle' })}>비밀번호 보호</div>
          </div>

          <Switch bind:checked={form.fields.hasPassword} />
        </div>

        {#if form.fields.hasPassword}
          <div class={flex({ position: 'relative' })}>
            <input
              class={css({
                borderWidth: '1px',
                borderRadius: '6px',
                paddingLeft: '12px',
                paddingRight: '32px',
                width: 'full',
                height: '32px',
                fontFamily: 'mono',
                fontSize: '12px',
                color: 'text.subtle',
              })}
              autocomplete="off"
              data-1p-ignore
              placeholder="비밀번호 입력"
              type={showPassword ? 'text' : 'password'}
              bind:value={form.fields.password}
            />

            <button
              class={center({
                position: 'absolute',
                top: '1/2',
                right: '8px',
                size: '20px',
                color: 'text.disabled',
                translate: 'auto',
                translateY: '-1/2',
                _hover: { color: 'text.disabled' },
              })}
              onclick={() => (showPassword = !showPassword)}
              type="button"
            >
              <Icon icon={showPassword ? EyeOffIcon : EyeIcon} size={16} />
            </button>
          </div>
        {/if}
      </div>

      <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={IdCardIcon} />
          <div class={css({ fontSize: '12px', color: 'text.subtle' })}>연령 제한</div>
        </div>

        <Select
          items={[
            { label: '없음', value: PostContentRating.ALL },
            { label: '15세', value: PostContentRating.R15 },
            { label: '성인', value: PostContentRating.R19 },
          ]}
          bind:value={form.fields.contentRating}
        />
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>포스트 상호작용</div>

      <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={SmileIcon} />
          <div class={css({ fontSize: '12px', color: 'text.subtle' })}>이모지 반응</div>
        </div>

        <Select
          items={[
            { icon: UsersRoundIcon, label: '누구나', value: true },
            { icon: BanIcon, label: '비허용', value: false },
          ]}
          bind:value={form.fields.allowReaction}
        />
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>포스트 보호</div>

      <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={ShieldIcon} />
          <div class={flex({ flexDirection: 'column' })}>
            <div class={css({ fontSize: '12px', color: 'text.subtle' })}>내용 보호</div>
            <p class={css({ fontSize: '10px', color: 'text.faint' })}>우클릭, 복사 및 다운로드 제한</p>
          </div>
        </div>

        <Switch bind:checked={form.fields.protectContent} />
      </div>
    </div>
  </div>
{:else if activeTab === 'share'}
  <div class={flex({ flexDirection: 'column', gap: '16px', paddingX: '16px', paddingTop: '16px', paddingBottom: '24px' })}>
    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>포스트 편집 권한</div>

      <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={BlendIcon} size={14} />
          <div class={css({ fontSize: '12px', color: 'text.subtle' })}>편집 범위</div>
        </div>

        <Select
          items={[
            {
              icon: LinkIcon,
              label: '링크가 있는 사람',
              description: '링크가 있는 누구나 편집할 수 있어요.',
              value: EntityAvailability.UNLISTED,
            },
            {
              icon: LockIcon,
              label: '나만',
              description: '나만 편집할 수 있어요.',
              value: EntityAvailability.PRIVATE,
            },
          ]}
          bind:value={form.fields.availability}
        />
      </div>
    </div>
  </div>
{/if}
