<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { isBodyEmpty } from '@typie/ui/tiptap';
  import ShapesIcon from '~icons/lucide/shapes';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';

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
  <div
    style:padding-left={`${paragraphIndent}em`}
    class={flex({
      position: 'relative',
      flexDirection: 'column',
      gap: '4px',
      width: 'full',
      maxWidth: 'var(--prosemirror-max-width)',
      color: 'text.disabled',
      lineHeight: '[1.6]',
      pointerEvents: 'none',
      zIndex: 'editor',
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
{/if}
