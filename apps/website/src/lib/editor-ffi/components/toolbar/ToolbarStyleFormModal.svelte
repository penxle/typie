<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Button, Modal, Select, TextInput } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import { untrack } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import { THEME_COLORS } from '$lib/editor-ffi/theme';
  import { values } from '$lib/editor-ffi/values';
  import { modifiersToCss } from './modifier-css';
  import ToolbarColorGrid from './ToolbarColorGrid.svelte';
  import type { Modifier } from '@typie/editor-ffi/browser';
  import type { ThemeVariant } from '$lib/editor-ffi/theme';

  type Font = { id?: string | null; weight: number; subfamilyDisplayName?: string | null; state: string };
  type FontFamily = { id: string; familyName: string; displayName: string; state: string; fonts: readonly Font[] };

  type Props = {
    open: boolean;
    mode?: 'create' | 'edit';
    initialName?: string;
    initialModifiers?: readonly Modifier[];
    fontFamilies?: readonly FontFamily[];
    onSubmit: (name: string, modifiers: Modifier[]) => void;
  };

  let { open = $bindable(false), mode = 'create', initialName = '', initialModifiers = [], fontFamilies = [], onSubmit }: Props = $props();

  const theme = getThemeContext();

  const themeVariant = $derived(
    (theme.effectiveTheme === 'light' ? `light-${theme.lightVariant}` : `dark-${theme.darkVariant}`) as ThemeVariant,
  );
  const tc = $derived(THEME_COLORS[themeVariant]);
  const textColorItems = $derived(values.textColor.map((c) => ({ label: c.label, value: c.value, color: tc[c.themeKey] })));
  const bgColorItems = $derived(
    values.textBackgroundColor.map((c) => ({ label: c.label, value: c.value, color: c.themeKey ? tc[c.themeKey] : null })),
  );
  const textColorMap = $derived(new Map<string, string>(values.textColor.map((c) => [c.value, tc[c.themeKey]])));
  const bgColorMap = $derived(
    new Map<string, string | null>(values.textBackgroundColor.map((c) => [c.value, c.themeKey ? tc[c.themeKey] : null])),
  );

  let name = $state('');
  let fontFamily = $state<string | undefined>();
  let fontSize = $state<number | undefined>();
  let fontWeight = $state<number | undefined>();
  let textColor = $state<string | undefined>();
  let backgroundColor = $state<string | undefined>();

  const findMod = <T extends Modifier['type']>(mods: readonly Modifier[], type: T) =>
    mods.find((m): m is Extract<Modifier, { type: T }> => m.type === type);

  $effect(() => {
    if (open) {
      untrack(() => {
        name = initialName;
        fontFamily = findMod(initialModifiers, 'font_family')?.value;
        fontSize = findMod(initialModifiers, 'font_size')?.value;
        fontWeight = findMod(initialModifiers, 'font_weight')?.value;
        textColor = findMod(initialModifiers, 'text_color')?.value;
        backgroundColor = findMod(initialModifiers, 'background_color')?.value;
      });
    }
  });

  const currentFontFamilyAndFonts = $derived.by(() => {
    if (fontFamily) {
      const family = fontFamilies.find((f) => f.familyName === fontFamily);
      if (family) {
        const fonts = [
          ...new Map(
            family.fonts
              .filter((f) => f.state === 'ACTIVE' || (fontWeight !== undefined && f.weight === fontWeight))
              .toSorted((a, b) => a.weight - b.weight)
              .map((f) => [f.weight, f]),
          ).values(),
        ];
        return { family: family.familyName, fonts };
      }
    }

    const fontsByWeight = new SvelteMap<number, Font>();
    for (const family of fontFamilies) {
      for (const font of family.fonts) {
        if (font.state === 'ACTIVE') {
          fontsByWeight.set(font.weight, font);
        }
      }
    }

    return {
      family: undefined as string | undefined,
      fonts: [...fontsByWeight.values()].toSorted((a, b) => a.weight - b.weight),
    };
  });

  const weightItems = $derived.by(() => {
    const items = currentFontFamilyAndFonts.fonts.map((font) => ({
      value: font.weight,
      label:
        values.fontWeight.find((f) => f.value === font.weight)?.label ??
        (font.subfamilyDisplayName ? `${font.subfamilyDisplayName} (${font.weight})` : String(font.weight)),
    }));

    if (fontWeight != null && !items.some((w) => w.value === fontWeight)) {
      items.push({
        value: fontWeight,
        label: values.fontWeight.find((f) => f.value === fontWeight)?.label ?? String(fontWeight),
      });
      items.sort((a, b) => a.value - b.value);
    }

    return items;
  });

  const previewModifiers = $derived.by(() => {
    const m: Modifier[] = [];
    if (fontFamily !== undefined) m.push({ type: 'font_family', value: fontFamily });
    if (fontSize !== undefined) m.push({ type: 'font_size', value: fontSize });
    if (fontWeight !== undefined) m.push({ type: 'font_weight', value: fontWeight });
    if (textColor !== undefined) m.push({ type: 'text_color', value: textColor });
    if (backgroundColor !== undefined) m.push({ type: 'background_color', value: backgroundColor });
    return m;
  });

  const previewStyle = $derived(modifiersToCss(previewModifiers, { textColorMap, bgColorMap, maxFontSize: 28 }));

  const handleSubmit = () => {
    onSubmit(name.trim() || '새 스타일', previewModifiers);
    open = false;
  };

  const title = $derived(mode === 'create' ? '새 스타일 만들기' : '스타일 수정');
  const description = $derived(mode === 'create' ? '현재 서식을 기반으로 새 스타일을 만들어요.' : '스타일의 이름과 서식을 수정해요.');
  const submitLabel = $derived(mode === 'create' ? '생성' : '저장');

  const fieldLabel = css.raw({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' });
</script>

<Modal style={css.raw({ padding: '24px', maxWidth: '480px' })} bind:open>
  <form
    class={flex({ flexDirection: 'column', gap: '20px' })}
    onsubmit={(e) => {
      e.preventDefault();
      handleSubmit();
    }}
  >
    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <div class={css({ fontSize: '15px', fontWeight: 'bold', letterSpacing: '-0.01em', color: 'text.default' })}>{title}</div>
      <div class={css({ fontSize: '13px', color: 'text.muted', wordBreak: 'keep-all' })}>{description}</div>
    </div>

    <div
      class={flex({
        alignItems: 'center',
        justifyContent: 'center',
        borderRadius: '8px',
        backgroundColor: 'surface.muted',
        height: '96px',
        paddingX: '16px',
        fontSize: '16px',
        color: 'text.default',
      })}
    >
      <p style={previewStyle}>미리보기 텍스트</p>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '6px' })}>
      <label class={css(fieldLabel)} for="style-name">스타일 이름</label>
      <TextInput id="style-name" autofocus placeholder="새 스타일" size="md" bind:value={name} />
    </div>

    <div class={grid({ columns: 3, gap: '12px' })}>
      <div class={flex({ flexDirection: 'column', gap: '6px', minWidth: '0' })}>
        <div class={css(fieldLabel)}>글꼴</div>
        <Select
          style={css.raw({ width: 'full', justifyContent: 'space-between' })}
          items={fontFamilies.map((f) => ({ value: f.familyName, label: f.displayName }))}
          onselect={(value) => {
            fontFamily = value;
          }}
          value={fontFamily ?? 'Pretendard'}
        />
      </div>
      <div class={flex({ flexDirection: 'column', gap: '6px', minWidth: '0' })}>
        <div class={css(fieldLabel)}>크기</div>
        <Select
          style={css.raw({ width: 'full', justifyContent: 'space-between' })}
          items={values.fontSize.map((s) => ({ value: s.value, label: `${s.label} px` }))}
          onselect={(value) => {
            fontSize = value;
          }}
          value={fontSize ?? 1600}
        />
      </div>
      <div class={flex({ flexDirection: 'column', gap: '6px', minWidth: '0' })}>
        <div class={css(fieldLabel)}>굵기</div>
        <Select
          style={css.raw({ width: 'full', justifyContent: 'space-between' })}
          items={weightItems}
          onselect={(value) => {
            fontWeight = value;
          }}
          value={fontWeight ?? 400}
        />
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <div class={css(fieldLabel)}>글자 색</div>
      <ToolbarColorGrid columns={11} currentValue={textColor} items={textColorItems} onSelect={(v) => (textColor = v)} opened={false} />
    </div>

    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <div class={css(fieldLabel)}>배경 색</div>
      <ToolbarColorGrid
        columns={8}
        currentValue={backgroundColor}
        items={bgColorItems}
        onSelect={(v) => (backgroundColor = v)}
        opened={false}
        shape="square"
      />
    </div>

    <div class={flex({ justifyContent: 'flex-end', gap: '10px' })}>
      <Button
        style={css.raw({ paddingX: '16px' })}
        onclick={() => {
          open = false;
        }}
        type="button"
        variant="secondary"
      >
        취소
      </Button>
      <Button style={css.raw({ paddingX: '16px' })} type="submit">{submitLabel}</Button>
    </div>
  </form>
</Modal>
