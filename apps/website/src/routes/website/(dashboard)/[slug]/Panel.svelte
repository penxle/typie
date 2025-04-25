<script lang="ts">
  import dayjs from 'dayjs';
  import { PostVisibility } from '@/enums';
  import { fragment, graphql } from '$graphql';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import PanelCharacterCountChangeWidget from './PanelCharacterCountChangeWidget.svelte';
  import PanelCharacterCountWidget from './PanelCharacterCountWidget.svelte';
  import Share from './Share.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Editor_Panel_post } from '$graphql';
  import type { Ref } from '$lib/utils';

  type Props = {
    $post: Editor_Panel_post;
    editor?: Ref<Editor>;
  };

  let { $post: _post, editor }: Props = $props();

  const post = fragment(
    _post,
    graphql(`
      fragment Editor_Panel_post on Post {
        id
        createdAt
        updatedAt

        option {
          id
          visibility
        }

        ...Editor_Panel_CharacterCountChangeWidget_post
        ...Editor_Share_post
      }
    `),
  );
</script>

<aside
  class={flex({
    flexDirection: 'column',
    flexShrink: 0,
    gap: '24px',
    borderLeftWidth: '1px',
    borderLeftColor: 'gray.100',
    padding: '20px',
    minWidth: '200px',
    width: '[15vw]',
    maxWidth: '240px',
  })}
>
  <div class={flex({ flexDirection: 'column', gap: '6px' })}>
    <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
      <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'gray.700' })}>공유 상태</div>

      <Share {$post} />
    </div>

    {#if $post.option.visibility === PostVisibility.UNLISTED}
      <div
        class={css({
          borderRadius: '4px',
          paddingX: '8px',
          paddingY: '4px',
          width: 'fit',
          fontSize: '12px',
          fontWeight: 'semibold',
          color: 'blue.500',
          backgroundColor: 'blue.100',
          userSelect: 'none',
        })}
      >
        링크 공개
      </div>
    {:else if $post.option.visibility === PostVisibility.PRIVATE}
      <div
        class={css({
          borderRadius: '4px',
          paddingX: '8px',
          paddingY: '4px',
          width: 'fit',
          fontSize: '12px',
          fontWeight: 'semibold',
          color: 'gray.600',
          backgroundColor: 'gray.200',
          userSelect: 'none',
        })}
      >
        비공개
      </div>
    {/if}
  </div>

  <div class={flex({ flexDirection: 'column', gap: '6px' })}>
    <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'gray.700' })}>최초 생성 시각</div>
    <div class={css({ fontSize: '13px', color: 'gray.700' })}>{dayjs($post.createdAt).formatAsDateTime()}</div>
  </div>

  <div class={flex({ flexDirection: 'column', gap: '6px' })}>
    <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'gray.700' })}>마지막 수정 시각</div>
    <div class={css({ fontSize: '13px', color: 'gray.700' })}>{dayjs($post.updatedAt).formatAsDateTime()}</div>
  </div>

  <div class={flex({ flexDirection: 'column', gap: '12px' })}>
    <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'gray.700' })}>본문 정보</div>

    <div class={flex({ flexDirection: 'column' })}>
      <PanelCharacterCountWidget {editor} />
      <PanelCharacterCountChangeWidget {$post} />
    </div>
  </div>
</aside>
