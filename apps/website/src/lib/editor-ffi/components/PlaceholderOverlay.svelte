<script lang="ts">
  import { defaultValues } from '@typie/lib/const';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import { getEditorContext } from '../editor.svelte';

  const { editor } = getEditorContext();

  const PT_TO_PX = 96 / 72;

  const placeholder = $derived(editor?.placeholder);
  const container = $derived(placeholder && editor ? editor.pageEls[placeholder.page_idx] : undefined);
  const show = $derived(!!placeholder && !!container && !editor?.readOnly);

  const top = $derived(placeholder?.rect.y ?? 0);
  const left = $derived(placeholder?.rect.x ?? 0);
  const width = $derived(placeholder?.rect.width ?? 0);

  // TODO(editor-placeholder): remove host fallbacks once all clients consume concrete engine metrics.
  const fontSize = $derived(placeholder?.font_size ?? defaultValues.fontSize);
  const lineHeight = $derived(placeholder?.line_height ?? defaultValues.lineHeight);
  const letterSpacing = $derived(placeholder?.letter_spacing ?? defaultValues.letterSpacing);
  const textAlign = $derived(placeholder?.align ?? defaultValues.textAlign);
  const alignItems = $derived(textAlign === 'center' ? 'center' : textAlign === 'right' ? 'flex-end' : 'flex-start');

  const fontSizePx = $derived(`${(fontSize / 100) * PT_TO_PX}px`);
  const letterSpacingEm = $derived(`${letterSpacing / 100}em`);
  const lineHeightRatio = $derived(String(lineHeight / 100));

  let element = $state<HTMLDivElement>();

  $effect(() => {
    if (show && container && element && element.parentElement !== container) {
      container.append(element);
    }
  });

  const loadTemplate = () => {
    window.dispatchEvent(new CustomEvent('open-document-template-modal'));
  };
</script>

<div
  bind:this={element}
  style:display={show ? 'flex' : 'none'}
  style:top={`${top}px`}
  style:left={`${left}px`}
  style:width={`${width}px`}
  class={flex({
    position: 'absolute',
    flexDirection: 'column',
    color: 'text.disabled',
    pointerEvents: 'none',
    userSelect: 'none',
  })}
>
  <div
    style:font-size={fontSizePx}
    style:letter-spacing={letterSpacingEm}
    style:line-height={lineHeightRatio}
    style:text-align={textAlign}
    style:align-items={alignItems}
    class={flex({ width: 'full', flexDirection: 'column', gap: '4px' })}
  >
    <div class={css({ width: 'full', whiteSpace: 'pre-line' })}>내용을 입력하거나</div>
    <button
      style:text-align={textAlign}
      class={css({
        display: 'inline-flex',
        alignItems: 'center',
        gap: '4px',
        transition: 'common',
        pointerEvents: 'auto',
        _hover: { color: 'text.faint' },
      })}
      data-external-element
      onclick={loadTemplate}
      onpointerdown={(e) => e.stopPropagation()}
      type="button"
    >
      <Icon style={css.raw({ flexShrink: '0', size: '[1em]' })} icon={LayoutTemplateIcon} />
      <span>템플릿 불러오기</span>
    </button>
  </div>
</div>
