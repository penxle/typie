<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { getAppContext, getThemeContext } from '@typie/ui/context';
  import MinusIcon from '~icons/lucide/minus';
  import SlashIcon from '~icons/lucide/slash';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { THEME_COLORS } from '$lib/editor-ffi/theme';
  import { values } from '$lib/editor-ffi/values';
  import { toolbarRecent } from './toolbar-recent.svelte';
  import ToolbarPanel from './ToolbarPanel.svelte';
  import ToolbarPanelDropdown from './ToolbarPanelDropdown.svelte';
  import type { ThemeVariant } from '$lib/editor-ffi/theme';
  import type { ToolbarPanelItem } from './ToolbarPanel.svelte';

  type Props = {
    kind: 'text' | 'background';
    disabled?: boolean;
  };

  let { kind, disabled = false }: Props = $props();

  const app = getAppContext();
  const theme = getThemeContext();
  const ctx = getEditorContext();
  const recent = toolbarRecent(app.userId);

  const recentKey = $derived(kind === 'text' ? 'textColor' : 'backgroundColor');
  const themeVariant = $derived(
    (theme.effectiveTheme === 'light' ? `light-${theme.lightVariant}` : `dark-${theme.darkVariant}`) as ThemeVariant,
  );
  const tc = $derived(THEME_COLORS[themeVariant]);

  const state = $derived(kind === 'text' ? ctx.editor?.modifierState?.text_color : ctx.editor?.modifierState?.background_color);
  const mixed = $derived(state?.type === 'mixed');
  const current = $derived(
    state?.type === 'uniform' ? state.value.value : state?.type === 'absent' ? (kind === 'text' ? 'black' : 'none') : undefined,
  );

  const items = $derived<ToolbarPanelItem[]>(
    kind === 'text'
      ? values.textColor.map((c) => ({
          id: c.value,
          label: c.label,
          keywords: [c.value],
          selected: current === c.value,
          swatch: { color: tc[c.themeKey], shape: 'circle' },
        }))
      : values.textBackgroundColor.map((c) => ({
          id: c.value,
          label: c.label,
          keywords: [c.value],
          selected: current === c.value,
          swatch: { color: c.themeKey ? tc[c.themeKey] : null, shape: 'square' },
        })),
  );

  const label = $derived(kind === 'text' ? (mixed ? '글씨 색: 여러 색' : '글씨 색') : mixed ? '배경색: 여러 색' : '배경색');
  const currentColor = $derived(items.find((item) => item.id === current)?.swatch?.color);

  const apply = (value: string, close: () => void) => {
    if (kind === 'text') {
      ctx.editor?.enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'text_color', value } } });
    } else if (value === 'none') {
      ctx.editor?.enqueue({ type: 'modifier', op: { type: 'edit', modifier_type: 'background_color', modifier: undefined } });
    } else {
      ctx.editor?.enqueue({ type: 'modifier', op: { type: 'set', modifier: { type: 'background_color', value } } });
    }
    recent.remember(recentKey, value);
    close();
    ctx.editor?.focus();
  };
</script>

<ToolbarPanelDropdown {disabled} {label}>
  {#snippet anchor()}
    <div class={center({ size: '16px' })}>
      <div
        style:background-color={mixed || current === 'none' ? 'transparent' : currentColor}
        class={center({ borderWidth: '1px', borderRadius: kind === 'text' ? 'full' : '[3px]', size: '14px', position: 'relative' })}
      >
        {#if mixed}
          <Icon style={css.raw({ color: 'text.muted' })} icon={MinusIcon} size={12} />
        {:else if current === 'none'}
          <Icon style={css.raw({ color: 'text.hint' })} icon={SlashIcon} size={10} />
        {/if}
      </div>
    </div>
  {/snippet}

  {#snippet panel({ close })}
    <ToolbarPanel
      {items}
      onselect={(id) => apply(id, close)}
      placeholder={kind === 'text' ? '글씨 색' : '배경색'}
      recentIds={recent.ids(recentKey)}
    />
  {/snippet}
</ToolbarPanelDropdown>
