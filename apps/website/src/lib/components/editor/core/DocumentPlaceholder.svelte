<script lang="ts">
  import { defaultValues } from '@typie/lib/const';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { portal } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import { getEditorContext } from '$lib/editor/context.svelte';

  const { editor } = getEditorContext();

  const PT_TO_PX = 96 / 72;

  const getNumericAttr = (type: 'font_size' | 'letter_spacing' | 'line_height') => {
    const values = editor.getAttr(type)?.values.filter((value): value is number => value != null) ?? [];
    return values.length === 1 ? values[0] : undefined;
  };

  const getTextAlignAttr = () => {
    const values =
      editor.getAttr('text_align')?.values.filter((value): value is 'left' | 'center' | 'right' | 'justify' => value != null) ?? [];
    return values.length === 1 ? values[0] : undefined;
  };

  const fontSize = $derived(getNumericAttr('font_size') ?? editor.defaultAttrs?.fontSize ?? defaultValues.fontSize);
  const letterSpacing = $derived(getNumericAttr('letter_spacing') ?? editor.defaultAttrs?.letterSpacing ?? defaultValues.letterSpacing);
  const lineHeight = $derived(getNumericAttr('line_height') ?? editor.defaultAttrs?.lineHeight ?? defaultValues.lineHeight);
  const textAlign = $derived(getTextAlignAttr() ?? editor.defaultAttrs?.textAlign ?? defaultValues.textAlign);
  const alignItems = $derived(textAlign === 'center' ? 'center' : textAlign === 'right' ? 'flex-end' : 'flex-start');
  const fontSizePx = $derived(`${(fontSize / 100) * PT_TO_PX}px`);
  const letterSpacingEm = $derived(`${letterSpacing / 100}em`);
  const lineHeightRatio = $derived(String(lineHeight / 100));
  const placeholderBounds = $derived(editor.placeholder.bounds);

  const loadTemplate = () => {
    window.dispatchEvent(new CustomEvent('open-document-template-modal'));
  };

  const shouldShow = $derived(editor.placeholder.visible && editor.placeholder.bounds && editor.pageContainerEls[0]);
</script>

{#if shouldShow}
  <div
    style:top={`${placeholderBounds?.y ?? 0}px`}
    style:left={`${placeholderBounds?.x ?? 0}px`}
    style:width={`${placeholderBounds?.width ?? 0}px`}
    class={flex({
      position: 'absolute',
      flexDirection: 'column',
      color: 'text.disabled',
      pointerEvents: 'none',
      userSelect: 'none',
    })}
    use:portal={editor.pageContainerEls[0]}
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
        type="button"
      >
        <Icon style={css.raw({ flexShrink: '0', size: '[1em]' })} icon={LayoutTemplateIcon} />
        <span>템플릿 불러오기</span>
      </button>
    </div>
  </div>
{/if}
