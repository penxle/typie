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
  import DocumentPanelCharacterCount from './DocumentPanelCharacterCount.svelte';
  import DocumentPanelCharacterCountChange from './DocumentPanelCharacterCountChange.svelte';
  import type { DocumentPanel_Info_document, DocumentPanel_Info_user } from '$graphql';
  import type { Editor } from '$lib/editor/editor.svelte';

  type Props = {
    $document: DocumentPanel_Info_document;
    $user: DocumentPanel_Info_user;
    editor: Editor;
  };

  let { $document: _document, $user: _user, editor }: Props = $props();

  const document = fragment(
    _document,
    graphql(`
      fragment DocumentPanel_Info_document on Document {
        id
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

        ...DocumentPanel_Info_CharacterCountChange_document
      }
    `),
  );

  const user = fragment(
    _user,
    graphql(`
      fragment DocumentPanel_Info_user on User {
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
        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>공유 및 게시</div>

          {#if $user.id === $document.entity.user.id}
            <a
              class={cx('group', center({ size: '20px' }))}
              href={$document.entity.url}
              rel="noopener noreferrer"
              target="_blank"
              use:tooltip={{ message: '사이트에서 열기' }}
            >
              <Icon style={css.raw({ color: 'text.faint', _groupHover: { color: 'text.subtle' } })} icon={ExternalLinkIcon} size={14} />
            </a>
          {/if}
        </div>

        {#if $user.id === $document.entity.user.id}
          <button
            class={css({
              fontSize: '13px',
              fontWeight: 'medium',
              color: 'text.faint',
              transition: 'common',
              _hover: { color: 'text.subtle' },
            })}
            onclick={() => {
              app.state.shareOpen = [$document.entity.id];
              mixpanel.track('open_post_share_modal', { via: 'panel' });
            }}
            type="button"
          >
            설정
          </button>
        {/if}
      </div>

      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        {#if $document.entity.visibility === EntityVisibility.PUBLIC}
          <div
            class={css({
              borderRadius: '4px',
              paddingX: '8px',
              paddingY: '4px',
              width: 'fit',
              fontSize: '12px',
              fontWeight: 'semibold',
              color: 'accent.success.default',
              backgroundColor: 'accent.success.subtle',
              userSelect: 'none',
            })}
          >
            공개 조회
          </div>
        {:else if $document.entity.visibility === EntityVisibility.UNLISTED}
          <div
            class={css({
              borderRadius: '4px',
              paddingX: '8px',
              paddingY: '4px',
              width: 'fit',
              fontSize: '12px',
              fontWeight: 'semibold',
              color: 'accent.brand.default',
              backgroundColor: 'accent.brand.subtle',
              userSelect: 'none',
            })}
          >
            링크 조회
          </div>
        {:else}
          <div
            class={css({
              borderRadius: '4px',
              paddingX: '8px',
              paddingY: '4px',
              width: 'fit',
              fontSize: '12px',
              fontWeight: 'semibold',
              color: 'text.muted',
              backgroundColor: 'surface.subtle',
              userSelect: 'none',
            })}
          >
            비공개
          </div>
        {/if}

        {#if $document.entity.availability === EntityAvailability.UNLISTED}
          <div
            class={css({
              borderRadius: '4px',
              paddingX: '8px',
              paddingY: '4px',
              width: 'fit',
              fontSize: '12px',
              fontWeight: 'semibold',
              color: 'accent.brand.default',
              backgroundColor: 'accent.brand.subtle',
              userSelect: 'none',
            })}
          >
            링크 편집
          </div>
        {:else}
          <div
            class={css({
              borderRadius: '4px',
              paddingX: '8px',
              paddingY: '4px',
              width: 'fit',
              fontSize: '12px',
              fontWeight: 'semibold',
              color: 'text.muted',
              backgroundColor: 'surface.subtle',
              userSelect: 'none',
            })}
          >
            나만 편집
          </div>
        {/if}
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '6px' })}>
      <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>최초 생성 시각</div>
      <div class={css({ fontSize: '13px', color: 'text.subtle' })}>{dayjs($document.createdAt).formatAsDateTime()}</div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '6px' })}>
      <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>마지막 수정 시각</div>
      <div class={css({ fontSize: '13px', color: 'text.subtle' })}>{dayjs($document.updatedAt).formatAsDateTime()}</div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>본문 정보</div>

      <div class={flex({ flexDirection: 'column' })}>
        <DocumentPanelCharacterCount {editor} />
        <DocumentPanelCharacterCountChange {$document} />
      </div>
    </div>
  </div>
</div>
