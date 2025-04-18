<script lang="ts">
  import { z } from 'zod';
  import { PostContentRating, PostVisibility } from '@/enums';
  import CheckIcon from '~icons/lucide/check';
  import CopyIcon from '~icons/lucide/copy';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import EyeIcon from '~icons/lucide/eye';
  import EyeOffIcon from '~icons/lucide/eye-off';
  import GlobeIcon from '~icons/lucide/globe';
  import LockIcon from '~icons/lucide/lock';
  import LockKeyholeIcon from '~icons/lucide/lock-keyhole';
  import MessageSquareIcon from '~icons/lucide/message-square';
  import ShieldIcon from '~icons/lucide/shield';
  import SmileIcon from '~icons/lucide/smile';
  import UsersIcon from '~icons/lucide/users';
  import { fragment, graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { Button, HorizontalDivider, Icon, SegmentButtons, Switch } from '$lib/components';
  import { createForm } from '$lib/form';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import type { Editor_Share_post } from '$graphql';

  type Props = {
    $post: Editor_Share_post;
  };

  let { $post: _post }: Props = $props();

  const post = fragment(
    _post,
    graphql(`
      fragment Editor_Share_post on Post {
        id

        entity {
          id
          url
        }

        option {
          id
          visibility
          password
          contentRating
          allowComment
          allowReaction
          protectContent
        }
      }
    `),
  );

  const updatePostOption = graphql(`
    mutation Editor_Share_UpdatePostOption_Mutation($input: UpdatePostOptionInput!) {
      updatePostOption(input: $input) {
        id
        visibility
        password
        contentRating
        allowComment
        allowReaction
        protectContent
      }
    }
  `);

  let open = $state(false);

  let linkInputEl = $state<HTMLInputElement>();
  let copied = $state(false);
  let copiedTimeout = $state<NodeJS.Timeout>();

  let showPassword = $state(false);

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom-end',
    offset: 4,
    onClickOutside: () => {
      open = false;
    },
  });

  const form = createForm({
    schema: z.object({
      visibility: z.nativeEnum(PostVisibility),
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
    },
    defaultValues: {
      visibility: $post.option.visibility,
      hasPassword: $post.option.password !== null,
      password: $post.option.password,
      contentRating: $post.option.contentRating,
      allowComment: $post.option.allowComment,
      allowReaction: $post.option.allowReaction,
      protectContent: $post.option.protectContent,
    },
  });

  const handleCopyLink = () => {
    if (!linkInputEl) {
      return;
    }

    navigator.clipboard.writeText(linkInputEl.value);

    if (copiedTimeout) {
      clearTimeout(copiedTimeout);
    }

    copied = true;
    copiedTimeout = setTimeout(() => (copied = false), 2000);
  };
</script>

<div class={flex({ alignItems: 'center', gap: '12px', userSelect: 'none' })}>
  {#if form.fields.visibility === PostVisibility.PRIVATE}
    <div class={center({ gap: '6px', borderRadius: 'full', paddingX: '10px', paddingY: '4px', backgroundColor: 'gray.100' })}>
      <div class={css({ size: '6px', borderRadius: 'full', bg: 'gray.500' })}></div>
      <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'gray.700' })}>비공개 중</div>
    </div>
  {:else if form.fields.visibility === PostVisibility.UNLISTED}
    <div class={center({ gap: '6px', borderRadius: 'full', paddingX: '10px', paddingY: '4px', backgroundColor: 'brand.100' })}>
      <div class={css({ size: '6px', borderRadius: 'full', bg: 'brand.500' })}></div>
      <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'brand.700' })}>링크 공개 중</div>
    </div>
  {/if}

  <div use:anchor>
    <Button onclick={() => (open = true)} size="sm" variant="secondary">공유</Button>
  </div>
</div>

{#if open}
  <div
    class={css({
      borderWidth: '1px',
      borderRadius: '12px',
      paddingX: '16px',
      paddingY: '16px',
      width: '360px',
      backgroundColor: 'white',
      boxShadow: 'xlarge',
      zIndex: '50',
    })}
    use:floating
  >
    <div class={flex({ flexDirection: 'column', gap: '16px' })}>
      <div class={flex({ flexDirection: 'column', gap: '12px' })}>
        <div class={flex({ justifyContent: 'space-between', alignItems: 'center', userSelect: 'none' })}>
          {#if form.fields.visibility === PostVisibility.PRIVATE}
            <div class={center({ gap: '6px', borderRadius: 'full', paddingX: '10px', paddingY: '4px', backgroundColor: 'gray.100' })}>
              <div class={css({ size: '6px', borderRadius: 'full', bg: 'gray.500' })}></div>
              <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'gray.700' })}>비공개 중</div>
            </div>
          {:else if form.fields.visibility === PostVisibility.UNLISTED}
            <div class={center({ gap: '6px', borderRadius: 'full', paddingX: '10px', paddingY: '4px', backgroundColor: 'brand.100' })}>
              <div class={css({ size: '6px', borderRadius: 'full', bg: 'brand.500' })}></div>
              <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'brand.700' })}>링크 공개 중</div>
            </div>
          {/if}

          {#if form.fields.visibility === PostVisibility.PRIVATE}
            <div class={center({ color: 'gray.500', fontSize: '12px' })}>이 포스트는 나만 볼 수 있어요</div>
          {:else if form.fields.visibility === PostVisibility.UNLISTED}
            <div class={center({ color: 'gray.500', fontSize: '12px' })}>링크가 있는 누구나 포스트를 볼 수 있어요</div>
          {/if}
        </div>

        <div
          class={cx(
            'group',
            flex({
              alignItems: 'center',
              gap: '4px',
              borderWidth: '1px',
              borderRadius: '6px',
              paddingX: '12px',
              height: '36px',
              backgroundColor: 'gray.50',
              _hover: {
                borderColor: 'brand.200',
              },
            }),
          )}
        >
          <input
            bind:this={linkInputEl}
            class={css({ flexGrow: '1', color: 'gray.600', fontSize: '12px', _groupHover: { color: 'gray.900' } })}
            onclick={() => linkInputEl?.select()}
            readonly
            value={$post.entity.url}
          />

          <button
            class={center({
              borderRadius: '6px',
              size: '20px',
              color: 'gray.500',
              _hover: { color: 'gray.700', backgroundColor: 'gray.200' },
            })}
            onclick={handleCopyLink}
            type="button"
          >
            <Icon data-floating-keep-open icon={copied ? CheckIcon : CopyIcon} size={14} />
          </button>

          <a
            class={center({
              borderRadius: '6px',
              size: '20px',
              color: 'gray.500',
              _hover: { color: 'gray.700', backgroundColor: 'gray.200' },
            })}
            href={$post.entity.url}
            rel="noopener noreferrer"
            target="_blank"
          >
            <Icon icon={ExternalLinkIcon} size={14} />
          </a>
        </div>
      </div>

      {#if form.fields.visibility === PostVisibility.PRIVATE}
        <Button
          style={css.raw({ height: '36px' })}
          data-floating-keep-open
          onclick={() => (form.fields.visibility = PostVisibility.UNLISTED)}
        >
          <div class={center({ gap: '6px' })}>
            <Icon icon={GlobeIcon} />
            <span>링크 공개로 전환</span>
          </div>
        </Button>
      {:else}
        <HorizontalDivider />

        <div class={flex({ flexDirection: 'column', gap: '12px' })}>
          <div class={flex({ flexDirection: 'column', gap: '8px' })}>
            <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Icon style={css.raw({ color: 'gray.500' })} icon={LockKeyholeIcon} />
                <div class={css({ fontSize: '12px' })}>비밀번호 보호</div>
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
                    fontSize: '12px',
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
                  <Icon data-floating-keep-open icon={showPassword ? EyeOffIcon : EyeIcon} size={16} />
                </button>
              </div>
            {/if}
          </div>

          <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <Icon style={css.raw({ color: 'gray.500' })} icon={UsersIcon} />
              <div class={css({ fontSize: '12px' })}>연령 제한</div>
            </div>

            <SegmentButtons
              style={css.raw({ width: '150px' })}
              items={[
                { value: PostContentRating.ALL, label: '없음' },
                { value: PostContentRating.R15, label: '15세' },
                { value: PostContentRating.R19, label: '성인' },
              ]}
              onselect={(value) => (form.fields.contentRating = value)}
              size="sm"
              value={form.fields.contentRating}
            />
          </div>
        </div>

        <HorizontalDivider />

        <div class={flex({ flexDirection: 'column', gap: '12px' })}>
          <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <Icon style={css.raw({ color: 'gray.500' })} icon={MessageSquareIcon} />
              <div class={css({ fontSize: '12px' })}>댓글</div>
            </div>

            <SegmentButtons
              style={css.raw({ width: '200px' })}
              items={[
                { value: true, label: '로그인한 누구나' },
                { value: false, label: '비허용' },
              ]}
              onselect={(value) => (form.fields.allowComment = value)}
              size="sm"
              value={form.fields.allowComment}
            />
          </div>

          <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <Icon style={css.raw({ color: 'gray.500' })} icon={SmileIcon} />
              <div class={css({ fontSize: '12px' })}>이모지 반응</div>
            </div>

            <SegmentButtons
              style={css.raw({ width: '150px' })}
              items={[
                { value: true, label: '누구나' },
                { value: false, label: '비허용' },
              ]}
              onselect={(value) => (form.fields.allowReaction = value)}
              size="sm"
              value={form.fields.allowReaction}
            />
          </div>

          <div class={flex({ alignItems: 'center', justifyContent: 'space-between' })}>
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <Icon style={css.raw({ color: 'gray.500' })} icon={ShieldIcon} />
              <div class={flex({ flexDirection: 'column' })}>
                <div class={css({ fontSize: '12px' })}>게시물 보호</div>
                <p class={css({ fontSize: '10px', color: 'gray.500' })}>우클릭, 복사 및 다운로드 제한</p>
              </div>
            </div>

            <Switch bind:checked={form.fields.protectContent} />
          </div>
        </div>

        <HorizontalDivider />

        <div class={flex({ justifyContent: 'flex-end' })}>
          <button
            class={center({ gap: '6px', color: 'gray.400', fontSize: '12px', _hover: { color: 'gray.500' } })}
            data-floating-keep-open
            onclick={() => (form.fields.visibility = PostVisibility.PRIVATE)}
            type="button"
          >
            <Icon icon={LockIcon} size={12} />
            비공개로 전환
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}
