<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { EntityAvailability, EntityVisibility } from '@/enums';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import { fragment, graphql } from '$graphql';
  import PanelCharacterCount from './PanelCharacterCount.svelte';
  import PanelCharacterCountChange from './PanelCharacterCountChange.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';
  import type { Editor_Panel_PanelInfo_post, Editor_Panel_PanelInfo_user } from '$graphql';

  type Props = {
    $post: Editor_Panel_PanelInfo_post;
    $user: Editor_Panel_PanelInfo_user;
    editor?: Ref<Editor>;
  };

  let { $post: _post, $user: _user, editor }: Props = $props();

  const post = fragment(
    _post,
    graphql(`
      fragment Editor_Panel_PanelInfo_post on Post {
        id
        type
        createdAt
        updatedAt

        entity {
          id
          url
          visibility
          availability

          user {
            id
          }
        }

        ...Editor_Panel_PanelInfo_CharacterCountChange_post
      }
    `),
  );

  const user = fragment(
    _user,
    graphql(`
      fragment Editor_Panel_PanelInfo_user on User {
        id
      }
    `),
  );

  const app = getAppContext();
</script>

<div
  class={flex({
    flexDirection: 'column',
    minWidth: 'var(--min-width)',
    width: 'var(--width)',
    maxWidth: 'var(--max-width)',
    height: 'full',
  })}
>
  <div
    class={flex({
      flexShrink: '0',
      alignItems: 'center',
      height: '41px',
      paddingX: '20px',
      fontSize: '13px',
      fontWeight: 'semibold',
      color: 'text.subtle',
      borderBottomWidth: '1px',
      borderColor: 'surface.muted',
    })}
  >
    정보
  </div>

  <div class={flex({ flexDirection: 'column', gap: '20px', paddingX: '20px', paddingY: '16px', overflowY: 'auto' })}>
    <div class={flex({ flexDirection: 'column', gap: '6px' })}>
      <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>공유 및 게시</div>

        {#if $user.id === $post.entity.user.id}
          <button
            class={css({
              fontSize: '13px',
              fontWeight: 'medium',
              color: 'text.faint',
              transition: 'common',
              _hover: { color: 'text.subtle' },
            })}
            onclick={() => {
              app.state.shareOpen = [$post.entity.id];
              mixpanel.track('open_post_share_modal', { via: 'panel' });
            }}
            type="button"
          >
            설정
          </button>
        {/if}
      </div>

      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        {#if $post.entity.visibility === EntityVisibility.UNLISTED || $post.entity.availability === EntityAvailability.UNLISTED}
          <div
            class={css({
              borderRadius: '4px',
              paddingX: '8px',
              paddingY: '4px',
              width: 'fit',
              fontSize: '12px',
              fontWeight: 'semibold',
              color: 'text.link',
              backgroundColor: { base: 'blue.100', _dark: 'dark.blue.900' },
              userSelect: 'none',
            })}
          >
            {#if $post.entity.visibility === EntityVisibility.UNLISTED && $post.entity.availability === EntityAvailability.UNLISTED}
              링크 조회/편집
            {:else if $post.entity.visibility === EntityVisibility.UNLISTED}
              링크 조회
            {:else if $post.entity.availability === EntityAvailability.UNLISTED}
              링크 편집
            {/if}
          </div>
        {:else if $post.entity.visibility === EntityVisibility.PRIVATE}
          <div
            class={css({
              borderRadius: '4px',
              paddingX: '8px',
              paddingY: '4px',
              width: 'fit',
              fontSize: '12px',
              fontWeight: 'semibold',
              color: 'text.muted',
              backgroundColor: 'interactive.hover',
              userSelect: 'none',
            })}
          >
            비공개
          </div>
        {/if}

        {#if $user.id === $post.entity.user.id}
          <a
            class={cx('group', center({ size: '20px' }))}
            href={$post.entity.url}
            rel="noopener noreferrer"
            target="_blank"
            use:tooltip={{ message: '사이트에서 열기' }}
          >
            <Icon style={css.raw({ color: 'text.faint', _groupHover: { color: 'text.subtle' } })} icon={ExternalLinkIcon} size={14} />
          </a>
        {/if}
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '6px' })}>
      <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>최초 생성 시각</div>
      <div class={css({ fontSize: '13px', color: 'text.subtle' })}>{dayjs($post.createdAt).formatAsDateTime()}</div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '6px' })}>
      <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>마지막 수정 시각</div>
      <div class={css({ fontSize: '13px', color: 'text.subtle' })}>{dayjs($post.updatedAt).formatAsDateTime()}</div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>본문 정보</div>

      <div class={flex({ flexDirection: 'column' })}>
        <PanelCharacterCount {editor} />
        <PanelCharacterCountChange {$post} />
      </div>
    </div>
  </div>
</div>
