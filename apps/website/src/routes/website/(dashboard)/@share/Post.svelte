<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { HorizontalDivider, Icon, Select, Switch } from '@typie/ui/components';
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
    $posts: DashboardLayout_Share_Post_post[];
  };

  let { $posts: _posts }: Props = $props();

  const posts = fragment(
    _posts,
    graphql(`
      fragment DashboardLayout_Share_Post_post on Post {
        id
        title
        password
        contentRating
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

  let activeTab = $state<'publish' | 'share'>('publish');

  const isSinglePost = $derived($posts.length === 1);
  const postIds = $derived($posts.map((p) => p.id));

  const updatePostsOption = graphql(`
    mutation DashboardLayout_Share_Post_UpdatePostsOption_Mutation($input: UpdatePostsOptionInput!) {
      updatePostsOption(input: $input) {
        id
        password
        contentRating
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

  const visibilityIndeterminate = $derived($posts.length > 1 && $posts.some((p) => p.entity.visibility !== $posts[0].entity.visibility));
  const availabilityIndeterminate = $derived(
    $posts.length > 1 && $posts.some((p) => p.entity.availability !== $posts[0].entity.availability),
  );

  const form = createForm({
    schema: z.object({
      availability: z.nativeEnum(EntityAvailability),
      visibility: z.nativeEnum(EntityVisibility),
      hasPassword: z.boolean(),
      password: z.string().nullish(),
      contentRating: z.nativeEnum(PostContentRating),
      allowReaction: z.boolean(),
      protectContent: z.boolean(),
    }),
    submitOn: 'change',
    onSubmit: async (data) => {
      if ($posts.length === 0) return;

      const dirtyFields = form.getDirtyFields();
      const updateData: {
        postIds: string[];
        availability?: EntityAvailability;
        visibility?: EntityVisibility;
        contentRating?: PostContentRating;
        allowReaction?: boolean;
        protectContent?: boolean;
        password?: string | null;
      } = { postIds };

      if ('availability' in dirtyFields) updateData.availability = data.availability;
      if ('visibility' in dirtyFields) updateData.visibility = data.visibility;
      if ('contentRating' in dirtyFields) updateData.contentRating = data.contentRating;
      if ('allowReaction' in dirtyFields) updateData.allowReaction = data.allowReaction;
      if ('protectContent' in dirtyFields) updateData.protectContent = data.protectContent;
      if ('hasPassword' in dirtyFields || 'password' in dirtyFields) updateData.password = data.hasPassword ? data.password : null;

      if (Object.keys(updateData).length > 1) {
        await updatePostsOption(updateData);

        mixpanel.track('update_post_option', {
          ...updateData,
          hasPassword: data.hasPassword,
          count: $posts.length,
        });
      }
    },
    defaultValues: {
      availability: $posts[0].entity.availability,
      visibility: $posts[0].entity.visibility,
      hasPassword: $posts[0].password !== null,
      password: $posts.length > 1 && $posts.some((p) => p.password !== $posts[0].password) ? null : $posts[0].password,
      contentRating: $posts[0].contentRating,
      allowReaction: $posts[0].allowReaction,
      protectContent: $posts[0].protectContent,
    },
  });

  $effect(() => {
    void form;
  });

  $effect(() => {
    return () => {
      if (timer) {
        clearTimeout(timer);
      }
    };
  });

  const handleCopyLink = () => {
    if ($posts.length === 0) return;

    const urls = $posts.map((p) => (activeTab === 'publish' ? p.entity.url : `${env.PUBLIC_WEBSITE_URL}/${p.entity.slug}`)).join('\n');
    navigator.clipboard.writeText(urls);
    mixpanel.track('copy_post_share_url', { tab: activeTab, count: $posts.length });

    if (timer) {
      clearTimeout(timer);
    }

    copied = true;
    timer = setTimeout(() => (copied = false), 2000);
  };
</script>

<div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px', paddingX: '16px', paddingY: '12px' })}>
  <div class={flex({ gap: '[0.5ch]', fontSize: '12px', fontWeight: 'medium' })}>
    <span class={css({ wordBreak: 'break-all', lineClamp: '1', fontWeight: 'semibold' })}>
      {isSinglePost ? $posts[0].title : `${$posts.length}개의 포스트`}
    </span>
    <span class={css({ flexShrink: '0' })}>공유 및 게시하기</span>
  </div>
  <button
    class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}
    onclick={handleCopyLink}
    type="button"
    use:tooltip={{
      message:
        activeTab === 'publish'
          ? visibilityIndeterminate
            ? null
            : form.fields.visibility === EntityVisibility.PRIVATE
              ? '지금은 링크가 있어도 나만 볼 수 있어요'
              : '링크가 있는 누구나 포스트를 볼 수 있어요'
          : availabilityIndeterminate
            ? null
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
      <div class={css({ fontSize: '12px', color: 'text.link' })}>
        {activeTab === 'publish' ? '조회' : '편집'}
        {isSinglePost ? '링크' : '링크 모두'} 복사
      </div>
    {/if}
  </button>
</div>

<HorizontalDivider />

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
          values={$posts.map((p) => p.entity.visibility)}
          bind:value={form.fields.visibility}
        />
      </div>

      <div class={flex({ flexDirection: 'column', gap: '8px' })}>
        <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={LockKeyholeIcon} />
            <div class={css({ fontSize: '12px', color: 'text.subtle' })}>비밀번호 보호</div>
          </div>

          <Switch values={$posts.map((p) => p.password !== null)} bind:checked={form.fields.hasPassword} />
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
          values={$posts.map((p) => p.contentRating)}
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
          values={$posts.map((p) => p.allowReaction)}
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

        <Switch values={$posts.map((p) => p.protectContent)} bind:checked={form.fields.protectContent} />
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
          values={$posts.map((p) => p.entity.availability)}
          bind:value={form.fields.availability}
        />
      </div>
    </div>
  </div>
{/if}
