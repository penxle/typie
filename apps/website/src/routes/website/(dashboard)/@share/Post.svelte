<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { EntityVisibility, PostContentRating } from '@/enums';
  import BanIcon from '~icons/lucide/ban';
  import BlendIcon from '~icons/lucide/blend';
  import CheckIcon from '~icons/lucide/check';
  import UserIcon from '~icons/lucide/circle-user-round';
  import EyeIcon from '~icons/lucide/eye';
  import EyeOffIcon from '~icons/lucide/eye-off';
  import IdCardIcon from '~icons/lucide/id-card';
  import LinkIcon from '~icons/lucide/link';
  import LockIcon from '~icons/lucide/lock';
  import LockKeyholeIcon from '~icons/lucide/lock-keyhole';
  import MessageSquareIcon from '~icons/lucide/message-square';
  import ShieldIcon from '~icons/lucide/shield';
  import SmileIcon from '~icons/lucide/smile';
  import UsersIcon from '~icons/lucide/users-round';
  import { fragment, graphql } from '$graphql';
  import { tooltip } from '$lib/actions';
  import { HorizontalDivider, Icon, Select, Switch } from '$lib/components';
  import { createForm } from '$lib/form';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import type { DashboardLayout_Share_Post_post } from '$graphql';

  type Props = {
    $post: DashboardLayout_Share_Post_post;
  };

  let { $post: _post }: Props = $props();

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
          visibility
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
        }
      }
    }
  `);

  let copied = $state(false);
  let timer: NodeJS.Timeout | undefined;

  let showPassword = $state(false);

  const form = createForm({
    schema: z.object({
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
        visibility: data.visibility,
        password: data.hasPassword ? data.password : null,
        contentRating: data.contentRating,
        allowComment: data.allowComment,
        allowReaction: data.allowReaction,
        protectContent: data.protectContent,
      });

      mixpanel.track('update_post_option', {
        visibility: data.visibility,
        hasPassword: data.hasPassword,
        contentRating: data.contentRating,
        allowComment: data.allowComment,
        allowReaction: data.allowReaction,
        protectContent: data.protectContent,
      });
    },
    defaultValues: {
      visibility: $post.entity.visibility,
      hasPassword: $post.password !== null,
      password: $post.password,
      contentRating: $post.contentRating,
      allowComment: $post.allowComment,
      allowReaction: $post.allowReaction,
      protectContent: $post.protectContent,
    },
  });

  const handleCopyLink = () => {
    navigator.clipboard.writeText($post.entity.url);
    mixpanel.track('copy_post_share_url');

    if (timer) {
      clearTimeout(timer);
    }

    copied = true;
    timer = setTimeout(() => (copied = false), 2000);
  };
</script>

<div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px', paddingX: '16px', paddingY: '12px' })}>
  <div class={flex({ gap: '[0.5ch]', fontSize: '12px', fontWeight: 'medium' })}>
    <span class={css({ wordBreak: 'break-all', lineClamp: '1' })}>{$post.title}</span>
    <span class={css({ flexShrink: '0' })}>공유하기</span>
  </div>

  <button
    class={flex({ alignItems: 'center', gap: '4px', flexShrink: '0' })}
    onclick={handleCopyLink}
    type="button"
    use:tooltip={{
      message:
        form.fields.visibility === EntityVisibility.PRIVATE
          ? '지금은 링크가 있어도 나만 볼 수 있어요'
          : '링크가 있는 누구나 포스트를 볼 수 있어요',
      placement: 'top',
      keepOnClick: true,
    }}
  >
    {#if copied}
      <Icon style={css.raw({ color: 'blue.600' })} icon={CheckIcon} size={12} />
      <div class={css({ fontSize: '12px', color: 'blue.600' })}>복사되었어요</div>
    {:else}
      <Icon style={css.raw({ color: 'blue.600' })} icon={LinkIcon} size={12} />
      <div class={css({ fontSize: '12px', color: 'blue.600' })}>링크 복사</div>
    {/if}
  </button>
</div>

<HorizontalDivider />

<div class={flex({ flexDirection: 'column', gap: '16px', paddingX: '16px', paddingTop: '16px', paddingBottom: '24px' })}>
  <div class={flex({ flexDirection: 'column', gap: '12px' })}>
    <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'gray.500' })}>포스트 조회 권한</div>

    <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon style={css.raw({ color: 'gray.500' })} icon={BlendIcon} size={14} />
        <div class={css({ fontSize: '12px', color: 'gray.700' })}>공개 범위</div>
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
          <Icon style={css.raw({ color: 'gray.500' })} icon={LockKeyholeIcon} />
          <div class={css({ fontSize: '12px', color: 'gray.700' })}>비밀번호 보호</div>
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
              color: 'gray.700',
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
              color: 'gray.300',
              translate: 'auto',
              translateY: '-1/2',
              _hover: { color: 'gray.400' },
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
        <Icon style={css.raw({ color: 'gray.500' })} icon={IdCardIcon} />
        <div class={css({ fontSize: '12px', color: 'gray.700' })}>연령 제한</div>
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
    <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'gray.500' })}>포스트 상호작용</div>

    <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon style={css.raw({ color: 'gray.500' })} icon={MessageSquareIcon} />
        <div class={css({ fontSize: '12px', color: 'gray.700' })}>댓글</div>
      </div>

      <Select
        items={[
          { icon: UserIcon, label: '로그인한 이용자', value: true },
          { icon: BanIcon, label: '비허용', value: false },
        ]}
        bind:value={form.fields.allowComment}
      />
    </div>

    <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon style={css.raw({ color: 'gray.500' })} icon={SmileIcon} />
        <div class={css({ fontSize: '12px', color: 'gray.700' })}>이모지 반응</div>
      </div>

      <Select
        items={[
          { icon: UsersIcon, label: '누구나', value: true },
          { icon: BanIcon, label: '비허용', value: false },
        ]}
        bind:value={form.fields.allowReaction}
      />
    </div>
  </div>

  <div class={flex({ flexDirection: 'column', gap: '12px' })}>
    <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'gray.500' })}>포스트 보호</div>

    <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <Icon style={css.raw({ color: 'gray.500' })} icon={ShieldIcon} />
        <div class={flex({ flexDirection: 'column' })}>
          <div class={css({ fontSize: '12px', color: 'gray.700' })}>내용 보호</div>
          <p class={css({ fontSize: '10px', color: 'gray.500' })}>우클릭, 복사 및 다운로드 제한</p>
        </div>
      </div>

      <Switch bind:checked={form.fields.protectContent} />
    </div>
  </div>
</div>
