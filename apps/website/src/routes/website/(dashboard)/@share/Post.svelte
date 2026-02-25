<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
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
  import Dice5Icon from '~icons/lucide/dice-5';
  import EyeIcon from '~icons/lucide/eye';
  import EyeOffIcon from '~icons/lucide/eye-off';
  import GlobeIcon from '~icons/lucide/globe';
  import IdCardIcon from '~icons/lucide/id-card';
  import ImageIcon from '~icons/lucide/image';
  import LinkIcon from '~icons/lucide/link';
  import LockIcon from '~icons/lucide/lock';
  import LockKeyholeIcon from '~icons/lucide/lock-keyhole';
  import ShieldIcon from '~icons/lucide/shield';
  import SmileIcon from '~icons/lucide/smile';
  import Trash2Icon from '~icons/lucide/trash-2';
  import UsersRoundIcon from '~icons/lucide/users-round';
  import { env } from '$env/dynamic/public';
  import { Img } from '$lib/components';
  import { uploadBlobAsImage } from '$lib/utils';
  import { graphql } from '$mearie';
  import type { DashboardLayout_Share_Post_post$key } from '$mearie';

  type Props = {
    posts$key: DashboardLayout_Share_Post_post$key[];
  };

  let { posts$key }: Props = $props();

  const posts = createFragment(
    graphql(`
      fragment DashboardLayout_Share_Post_post on Post {
        id
        title
        password
        contentRating
        allowReaction
        protectContent

        thumbnail {
          id
          ...Img_image
        }

        entity {
          id
          url
          slug
          visibility
          availability
        }
      }
    `),
    () => posts$key,
  );

  let activeTab = $state<'publish' | 'share'>('publish');

  const isSinglePost = $derived(posts.data.length === 1);
  const postIds = $derived(posts.data.map((p) => p.id));

  const [updatePostsOption] = createMutation(
    graphql(`
      mutation DashboardLayout_Share_Post_UpdatePostsOption_Mutation($input: UpdatePostsOptionInput!) {
        updatePostsOption(input: $input) {
          id
          password
          contentRating
          allowReaction
          protectContent

          thumbnail {
            id
            ...Img_image
          }

          entity {
            id
            visibility
            availability
          }
        }
      }
    `),
  );

  let copied = $state(false);
  let timer: NodeJS.Timeout | undefined;

  let showPassword = $state(false);
  let isRolling = $state(false);
  let thumbnailUploading = $state(false);

  const visibilityIndeterminate = $derived(
    posts.data.length > 1 && posts.data.some((p) => p.entity.visibility !== posts.data[0].entity.visibility),
  );
  const availabilityIndeterminate = $derived(
    posts.data.length > 1 && posts.data.some((p) => p.entity.availability !== posts.data[0].entity.availability),
  );
  const thumbnailIndeterminate = $derived(posts.data.length > 1 && posts.data.some((p) => p.thumbnail?.id !== posts.data[0].thumbnail?.id));

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
      if (posts.data.length === 0) return;

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
        await updatePostsOption({ input: updateData });

        mixpanel.track('update_post_option', {
          ...updateData,
          hasPassword: data.hasPassword,
          count: posts.data.length,
        });
      }
    },
    defaultValues: {
      availability: posts.data[0].entity.availability,
      visibility: posts.data[0].entity.visibility,
      hasPassword: posts.data[0].password !== null,
      password: posts.data.length > 1 && posts.data.some((p) => p.password !== posts.data[0].password) ? null : posts.data[0].password,
      contentRating: posts.data[0].contentRating,
      allowReaction: posts.data[0].allowReaction,
      protectContent: posts.data[0].protectContent,
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

  const generateRandomPassword = () => {
    isRolling = true;

    const digits = '0123456789';
    let password = '';
    for (let i = 0; i < 4; i++) {
      password += digits.charAt(Math.floor(Math.random() * digits.length));
    }
    form.fields.password = password;
    showPassword = true;

    setTimeout(() => {
      isRolling = false;
    }, 500);
  };

  const handleCopyLink = () => {
    if (posts.data.length === 0) return;

    const urls = posts.data.map((p) => (activeTab === 'publish' ? p.entity.url : `${env.PUBLIC_WEBSITE_URL}/${p.entity.slug}`)).join('\n');
    navigator.clipboard.writeText(urls);
    mixpanel.track('copy_post_share_url', { tab: activeTab, count: posts.data.length });

    if (timer) {
      clearTimeout(timer);
    }

    copied = true;
    timer = setTimeout(() => (copied = false), 2000);
  };

  const handleThumbnailUpload = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';

    input.addEventListener('change', async () => {
      const file = input.files?.[0];
      if (!file) return;

      thumbnailUploading = true;
      try {
        const image = await uploadBlobAsImage(file);
        await updatePostsOption({ input: { postIds, thumbnailId: image.id } });
        mixpanel.track('update_post_thumbnail', { count: posts.data.length });
      } finally {
        thumbnailUploading = false;
      }
    });

    input.click();
  };

  const handleThumbnailRemove = async () => {
    await updatePostsOption({ input: { postIds, thumbnailId: null } });
    mixpanel.track('remove_post_thumbnail', { count: posts.data.length });
  };
</script>

<div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px', paddingX: '16px', paddingY: '12px' })}>
  <div class={flex({ gap: '[0.5ch]', fontSize: '12px', fontWeight: 'medium' })}>
    <span class={css({ wordBreak: 'break-all', lineClamp: '1', fontWeight: 'semibold' })}>
      {isSinglePost ? posts.data[0].title : `${posts.data.length}개의 포스트`}
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
              icon: GlobeIcon,
              label: '공개',
              description: '누구나 볼 수 있어요.',
              value: EntityVisibility.PUBLIC,
            },
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
          values={posts.data.map((p) => p.entity.visibility)}
          bind:value={form.fields.visibility}
        />
      </div>

      <div class={flex({ flexDirection: 'column', gap: '8px' })}>
        <div class={flex({ alignItems: 'center', justifyContent: 'space-between', height: '24px' })}>
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={LockKeyholeIcon} />
            <div class={css({ fontSize: '12px', color: 'text.subtle' })}>비밀번호 보호</div>
          </div>

          <Switch values={posts.data.map((p) => p.password !== null)} bind:checked={form.fields.hasPassword} />
        </div>

        {#if form.fields.hasPassword}
          <div class={flex({ position: 'relative' })}>
            <input
              class={css({
                borderWidth: '1px',
                borderRadius: '6px',
                paddingLeft: '12px',
                paddingRight: '56px',
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
                right: '32px',
                size: '20px',
                color: 'text.disabled',
                userSelect: 'none',
                translate: 'auto',
                translateY: '-1/2',
                _hover: { color: 'text.disabled' },
              })}
              onclick={generateRandomPassword}
              type="button"
              use:tooltip={{
                message: '4자리 랜덤 비밀번호 생성',
                placement: 'bottom',
              }}
            >
              <Icon
                class={css({
                  animation: isRolling ? 'diceRoll 0.5s cubic-bezier(0.4, 0.0, 0.2, 1)' : 'none',
                  transformOrigin: 'center',
                })}
                icon={Dice5Icon}
                size={16}
              />
            </button>

            <button
              class={center({
                position: 'absolute',
                top: '1/2',
                right: '8px',
                size: '20px',
                color: 'text.disabled',
                userSelect: 'none',
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
          values={posts.data.map((p) => p.contentRating)}
          bind:value={form.fields.contentRating}
        />
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.faint' })}>썸네일</div>

      <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={ImageIcon} />
          <div class={css({ fontSize: '12px', color: 'text.subtle' })}>미리보기 이미지</div>
        </div>

        <div class={flex({ gap: '8px', alignItems: 'center' })}>
          {#if thumbnailIndeterminate}
            <button
              class={center({
                width: '64px',
                height: '36px',
                borderRadius: '6px',
                backgroundColor: 'surface.muted',
                fontSize: '10px',
                color: 'text.faint',
              })}
              disabled={thumbnailUploading}
              onclick={handleThumbnailUpload}
              type="button"
            >
              {thumbnailUploading ? '...' : '다름'}
            </button>
          {:else if posts.data[0].thumbnail}
            <div class={flex({ alignItems: 'center', gap: '4px' })}>
              <button
                class={css({ position: 'relative', cursor: 'pointer' })}
                disabled={thumbnailUploading}
                onclick={handleThumbnailUpload}
                type="button"
              >
                <Img
                  style={css.raw({
                    width: '64px',
                    height: '36px',
                    borderRadius: '6px',
                    objectFit: 'cover',
                  })}
                  alt="썸네일"
                  image$key={posts.data[0].thumbnail}
                  size={128}
                />
              </button>
              <button
                class={center({
                  size: '24px',
                  borderRadius: '4px',
                  color: 'text.faint',
                  _hover: { backgroundColor: 'surface.muted', color: 'text.danger' },
                })}
                onclick={handleThumbnailRemove}
                type="button"
                use:tooltip={{ message: '삭제', placement: 'top' }}
              >
                <Icon icon={Trash2Icon} size={14} />
              </button>
            </div>
          {:else}
            <button
              class={center({
                width: '64px',
                height: '36px',
                borderWidth: '1px',
                borderStyle: 'dashed',
                borderRadius: '6px',
                color: 'text.faint',
                _hover: { backgroundColor: 'surface.muted' },
              })}
              disabled={thumbnailUploading}
              onclick={handleThumbnailUpload}
              type="button"
            >
              {#if thumbnailUploading}
                <span class={css({ fontSize: '10px' })}>...</span>
              {:else}
                <Icon icon={ImageIcon} size={14} />
              {/if}
            </button>
          {/if}
        </div>
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
          values={posts.data.map((p) => p.allowReaction)}
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

        <Switch values={posts.data.map((p) => p.protectContent)} bind:checked={form.fields.protectContent} />
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
          values={posts.data.map((p) => p.entity.availability)}
          bind:value={form.fields.availability}
        />
      </div>
    </div>
  </div>
{/if}
