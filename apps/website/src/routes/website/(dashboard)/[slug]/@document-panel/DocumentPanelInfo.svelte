<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import dayjs from 'dayjs';
  import { fragment, graphql } from '$graphql';
  import DocumentPanelCharacterCountChange from './DocumentPanelCharacterCountChange.svelte';
  import type { DocumentPanel_Info_document } from '$graphql';

  type Props = {
    $document: DocumentPanel_Info_document;
  };

  let { $document: _document }: Props = $props();

  const document = fragment(
    _document,
    graphql(`
      fragment DocumentPanel_Info_document on Document {
        id
        createdAt
        updatedAt

        ...DocumentPanel_Info_CharacterCountChange_document
      }
    `),
  );
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
        <DocumentPanelCharacterCountChange {$document} />
      </div>
    </div>
  </div>
</div>
