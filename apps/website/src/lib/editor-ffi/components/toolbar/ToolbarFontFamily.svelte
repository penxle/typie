<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getAppContext } from '@typie/ui/context';
  import { SvelteMap } from 'svelte/reactivity';
  import PlusIcon from '~icons/lucide/plus';
  import { FontSpecimen } from '$lib/components';
  import { familySpecimenFallbacks } from '$lib/components/font-specimen';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { toolbarRecent } from './toolbar-recent.svelte';
  import ToolbarPanel from './ToolbarPanel.svelte';
  import ToolbarPanelDropdown from './ToolbarPanelDropdown.svelte';
  import type { ToolbarPanelItem } from './ToolbarPanel.svelte';

  type Font = { id?: string | null; weight: number; subfamilyDisplayName?: string | null; state: string };
  type FontFamily = { id: string; familyName: string; displayName: string; state: string; fonts: readonly Font[] };

  type Props = {
    fontFamilies?: readonly FontFamily[];
    onUploadClick?: () => void;
    disabled?: boolean;
  };

  let { fontFamilies = [], onUploadClick, disabled = false }: Props = $props();

  const app = getAppContext();
  const ctx = getEditorContext();
  const recent = toolbarRecent(app.userId);

  const currentFontFamilyValue = $derived(
    ctx.editor?.modifierState?.font_family?.type === 'uniform' ? ctx.editor.modifierState.font_family.value.value : undefined,
  );

  const activeFamilies = $derived(fontFamilies.filter((f) => f.state === 'ACTIVE'));

  const fontFamilyItems = $derived.by(() => {
    if (currentFontFamilyValue && activeFamilies.every((f) => f.familyName !== currentFontFamilyValue)) {
      const current = fontFamilies.find((f) => f.familyName === currentFontFamilyValue);
      if (current) {
        return [...activeFamilies, current];
      }
    }
    return activeFamilies;
  });

  const representativeFontMap = $derived.by(() => {
    const map = new SvelteMap<string, Font | null>();
    for (const family of fontFamilies) {
      const active = family.fonts.filter((f) => f.state === 'ACTIVE');
      if (active.length === 0) {
        map.set(family.familyName, null);
      } else {
        map.set(
          family.familyName,
          active.reduce((prev, curr) => {
            const prevDiff = Math.abs(prev.weight - 400);
            const currDiff = Math.abs(curr.weight - 400);
            if (currDiff < prevDiff) return curr;
            if (currDiff === prevDiff && curr.weight > prev.weight) return curr;
            return prev;
          }),
        );
      }
    }
    return map;
  });

  const items = $derived<ToolbarPanelItem[]>(
    fontFamilyItems.map((f) => ({
      id: f.familyName,
      label: f.displayName,
      keywords: [f.familyName],
      selected: f.familyName === currentFontFamilyValue,
    })),
  );

  const currentLabel = $derived(
    currentFontFamilyValue === undefined
      ? '-'
      : (fontFamilies.find((f) => f.familyName === currentFontFamilyValue)?.displayName ?? '(알 수 없는 폰트)'),
  );

  const apply = (familyName: string, close: () => void) => {
    ctx.editor?.enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'font_family', value: familyName } } });
    recent.remember('fontFamily', familyName);
    close();
    ctx.editor?.focus();
  };
</script>

<ToolbarPanelDropdown style={css.raw({ maxWidth: '180px' })} chevron {disabled} label="폰트 패밀리">
  {#snippet anchor()}
    <span class={css({ paddingLeft: '2px', fontSize: '13px', fontWeight: 'medium', whiteSpace: 'nowrap', truncate: true })}>
      {currentLabel}
    </span>
  {/snippet}

  {#snippet panel({ close })}
    <ToolbarPanel
      actions={onUploadClick ? [{ id: 'upload', label: '직접 업로드', icon: PlusIcon }] : []}
      {items}
      onActionSelect={() => {
        close();
        onUploadClick?.();
      }}
      onselect={(id) => apply(id, close)}
      placeholder="폰트 패밀리"
      recentIds={recent.ids('fontFamily')}
      rowHeight={32}
    >
      {#snippet render(item)}
        {@const font = representativeFontMap.get(item.id)}
        <span class={css({ minWidth: '0', overflow: 'hidden', whiteSpace: 'nowrap' })}>
          <FontSpecimen
            fallbacks={familySpecimenFallbacks(item.label, item.id)}
            fontId={font?.id ?? undefined}
            text={item.label}
            weight={font?.weight}
          />
        </span>
      {/snippet}
    </ToolbarPanel>
  {/snippet}
</ToolbarPanelDropdown>
