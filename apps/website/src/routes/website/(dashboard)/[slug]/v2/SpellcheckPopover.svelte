<script lang="ts">
  import { hide, inline, shift, size } from '@floating-ui/dom';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import XIcon from '~icons/lucide/x';
  import { pageRectsToVirtualElement } from '$lib/editor-ffi/geometry';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';

  type Props = {
    editor: Editor;
  };

  let { editor }: Props = $props();

  const activeError = $derived(
    editor.activeSpellcheckErrorId ? editor.spellcheckErrors.find((e) => e.id === editor.activeSpellcheckErrorId) : undefined,
  );

  const activeRange = $derived(activeError ? editor.trackedRanges.find((r) => r.id === activeError.id) : undefined);

  const scroller = $derived(editor.scrollContainerEl);

  const { anchor, floating } = createFloatingActions({
    placement: 'top',
    offset: 4,
    middleware: [
      inline(),
      shift({
        get boundary() {
          return scroller ?? undefined;
        },
        padding: 8,
      }),
      size({
        get boundary() {
          return scroller ?? undefined;
        },
        padding: 8,
        apply({ availableWidth, elements }) {
          Object.assign(elements.floating.style, {
            maxWidth: `${availableWidth}px`,
          });
        },
      }),
      hide({
        strategy: 'escaped',
        get boundary() {
          return scroller ?? undefined;
        },
        padding: 32,
      }),
    ],
  });

  $effect(() => {
    if (activeRange && activeRange.rects.length > 0 && scroller) {
      anchor(pageRectsToVirtualElement(editor, activeRange.rects));
    }
  });

  const apply = (correction: string) => {
    if (!activeError) return;
    if (editor.readOnly) {
      Toast.error('잠긴 문서는 편집할 수 없어요.');
      editor.focus();
      return;
    }
    editor.applySpellcheckCorrection(activeError.id, correction);
    editor.focus();
  };

  const dismiss = () => {
    if (!activeError) return;
    editor.removeSpellcheckError(activeError.id);
    editor.focus();
  };
</script>

{#if activeError && activeRange && activeRange.rects.length > 0 && scroller}
  <div
    class={flex({
      alignItems: 'center',
      gap: '4px',
      zIndex: 'overEditor',
      flexWrap: 'wrap',
      pointerEvents: 'auto',
    })}
    use:floating={{ appendTo: scroller }}
  >
    {#each activeError.corrections as correction (correction)}
      <button
        class={flex({
          justifyContent: 'space-between',
          alignItems: 'center',
          gap: '4px',
          borderWidth: '1px',
          borderColor: 'border.danger',
          borderRadius: '4px',
          paddingX: '8px',
          paddingY: '4px',
          fontSize: '13px',
          fontWeight: 'semibold',
          color: 'text.danger',
          backgroundColor: 'accent.danger.subtle',
          transition: 'common',
          boxShadow: 'small',
          _hover: { backgroundColor: { base: 'red.100', _dark: 'dark.red.800' } },
        })}
        onclick={() => apply(correction)}
        type="button"
      >
        {correction}
        <Icon icon={ArrowRightIcon} size={12} />
      </button>
    {/each}

    <button
      class={flex({
        alignItems: 'center',
        justifyContent: 'center',
        size: '29px',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '4px',
        backgroundColor: 'surface.default',
        color: 'text.faint',
        transition: 'common',
        boxShadow: 'small',
        _hover: { backgroundColor: 'surface.muted', color: 'text.subtle' },
      })}
      onclick={dismiss}
      type="button"
    >
      <Icon icon={XIcon} size={14} />
    </button>
  </div>
{/if}
