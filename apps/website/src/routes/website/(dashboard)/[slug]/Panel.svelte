<script lang="ts">
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import { EntityAvailability, EntityVisibility, PostType } from '@/enums';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import { fragment, graphql } from '$graphql';
  import { tooltip } from '$lib/actions';
  import { HorizontalDivider, Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import PanelCharacterCountChangeWidget from './PanelCharacterCountChangeWidget.svelte';
  import PanelCharacterCountWidget from './PanelCharacterCountWidget.svelte';
  import PanelNote from './PanelNote.svelte';
  import type { Editor } from '@tiptap/core';
  import type * as Y from 'yjs';
  import type { Editor_Panel_post, Editor_Panel_user } from '$graphql';
  import type { Ref } from '$lib/utils';

  type Props = {
    $post: Editor_Panel_post;
    $user: Editor_Panel_user;
    editor?: Ref<Editor>;
    doc: Y.Doc;
  };

  let { $post: _post, $user: _user, editor, doc }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment Editor_Panel_user on User {
        id
      }
    `),
  );

  const post = fragment(
    _post,
    graphql(`
      fragment Editor_Panel_post on Post {
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

        ...Editor_Panel_CharacterCountChangeWidget_post
      }
    `),
  );

  const app = getAppContext();
</script>

<aside
  style:--min-width="220px"
  style:--width="15vw"
  style:--max-width="240px"
  class={css({
    flexShrink: '0',
    minWidth: app.preference.current.panelExpanded ? 'var(--min-width)' : '0',
    width: 'var(--width)',
    maxWidth: app.preference.current.panelExpanded ? 'var(--max-width)' : '0',
    opacity: app.preference.current.panelExpanded ? '100' : '0',
    transitionProperty: 'min-width, max-width, opacity',
    transitionDuration: '200ms',
    transitionTimingFunction: 'ease',
    willChange: 'min-width, max-width, opacity',
    overflowX: 'hidden',
  })}
>
  <div
    class={flex({
      flexDirection: 'column',
      gap: '16px',
      borderLeftWidth: '1px',
      borderColor: 'border.subtle',
      paddingTop: '16px',
      minWidth: 'var(--min-width)',
      width: 'var(--width)',
      maxWidth: 'var(--max-width)',
      height: 'full',
    })}
  >
    <div class={flex({ flexDirection: 'column', gap: '6px', paddingX: '20px' })}>
      <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
        <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>
          {#if $post.type === PostType.NORMAL}
            포스트
          {:else if $post.type === PostType.TEMPLATE}
            템플릿
          {/if}
        </div>

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

    <HorizontalDivider color="secondary" />

    <div class={flex({ flexDirection: 'column', gap: '20px', paddingX: '20px' })}>
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
                app.state.shareOpen = $post.entity.id;
                mixpanel.track('open_post_share_modal', { via: 'panel' });
              }}
              type="button"
            >
              설정
            </button>
          {/if}
        </div>

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
              backgroundColor: { base: 'blue.100', _dark: 'blue.950' },
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
          <PanelCharacterCountWidget {editor} />
          <PanelCharacterCountChangeWidget {$post} />
        </div>
      </div>
    </div>

    <HorizontalDivider color="secondary" />

    {#if !app.preference.current.noteExpanded}
      <PanelNote {doc} />
    {/if}
  </div>
</aside>
