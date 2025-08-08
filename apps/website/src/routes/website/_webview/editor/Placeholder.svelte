<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import ShapesIcon from '~icons/lucide/shapes';
  import { Icon } from '$lib/components';
  import { isBodyEmpty } from '$lib/tiptap';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor: Ref<Editor>;
    isTemplateActive: boolean;
  };

  let { editor, isTemplateActive }: Props = $props();

  const emptyBody = $derived(isBodyEmpty(editor.current.state));

  const paragraphIndent = $derived.by(() => {
    const { doc } = editor.current.state;
    const body = doc.child(0);
    return body.attrs.paragraphIndent;
  });
</script>

{#if emptyBody}
  <div class={center({ position: 'absolute', top: '0', insetX: '0', flexGrow: '1', pointerEvents: 'none' })}>
    <div
      style:padding-left={`${paragraphIndent}em`}
      class={flex({
        flexDirection: 'column',
        gap: '4px',
        width: 'full',
        maxWidth: 'var(--prosemirror-max-width)',
        color: 'text.disabled',
        lineHeight: '[1.6]',
      })}
    >
      <div class={css({ fontFamily: 'ui' })}>내용</div>

      {#if isTemplateActive}
        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          <div>혹은</div>
          <button
            class={flex({
              alignItems: 'center',
              gap: '4px',
              pointerEvents: 'auto',
            })}
            onclick={() => {
              window.__webview__?.emitEvent('useTemplate');
            }}
            type="button"
          >
            <Icon icon={ShapesIcon} size={16} />
            <div>템플릿 사용하기</div>
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}
