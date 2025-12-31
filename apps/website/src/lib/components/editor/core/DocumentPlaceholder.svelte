<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { portal } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import { getEditor } from '$lib/editor/context';

  const editor = getEditor();

  let element = $state<HTMLDivElement>();

  const loadTemplate = () => {
    window.dispatchEvent(new CustomEvent('open-document-template-modal'));
  };

  $effect(() => {
    if (!element) return;

    const { visible, bounds } = editor.placeholder;

    if (visible && bounds) {
      element.style.top = `${bounds.y}px`;
      element.style.left = `${bounds.x}px`;
      element.style.width = `${bounds.width}px`;
    }
  });

  const shouldShow = $derived(editor.placeholder.visible && editor.placeholder.bounds);
</script>

{#if shouldShow}
  <div
    bind:this={element}
    class={flex({
      position: 'absolute',
      flexDirection: 'column',
      alignItems: 'flex-start',
      gap: '4px',
      color: 'text.disabled',
      pointerEvents: 'none',
      userSelect: 'none',
    })}
    use:portal={editor.pageContainerEls[0]}
  >
    <div class={css({ whiteSpace: 'pre-line' })}>내용을 입력하거나</div>
    <button
      class={css({
        textAlign: 'start',
        transition: 'common',
        pointerEvents: 'auto',
        _hover: { color: 'text.faint' },
      })}
      data-external-element
      onclick={loadTemplate}
      type="button"
    >
      <Icon style={{ display: 'inline-block', marginRight: '4px', marginBottom: '3px' }} icon={LayoutTemplateIcon} size={16} />
      <span>템플릿 불러오기</span>
    </button>
  </div>
{/if}
