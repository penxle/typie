<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { DARK_VARIANTS, LIGHT_VARIANTS, VARIANT_LABELS, VARIANT_SELECTION } from '@typie/styled-system/presets';
  import { Icon } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import CheckIcon from '~icons/lucide/check';
  import type { DarkVariant, LightVariant } from '@typie/ui/context';

  const theme = getThemeContext();

  function selectLightVariant(variant: LightVariant) {
    theme.overrideTheme = 'light';
    theme.lightVariant = variant;
    mixpanel.track('change_theme_variant', { mode: 'light', variant });
  }

  function selectDarkVariant(variant: DarkVariant) {
    theme.overrideTheme = 'dark';
    theme.darkVariant = variant;
    mixpanel.track('change_theme_variant', { mode: 'dark', variant });
  }

  $effect(() => {
    return () => {
      theme.overrideTheme = undefined;
    };
  });
</script>

{#snippet preview(selection: string)}
  <div class={flex({ height: '72px', width: 'full', borderBottomWidth: '1px', borderColor: 'border.hairline' })}>
    <div
      class={flex({
        direction: 'column',
        gap: '5px',
        width: '[28%]',
        height: 'full',
        paddingX: '6px',
        paddingY: '8px',
        backgroundColor: 'surface.canvas',
        borderRightWidth: '1px',
        borderColor: 'border.hairline',
      })}
    >
      <div class={css({ height: '4px', width: '[70%]', borderRadius: 'full', backgroundColor: 'text.hint' })}></div>
      <div class={css({ height: '8px', width: 'full', borderRadius: '2px', backgroundColor: 'surface.active' })}></div>
    </div>
    <div
      class={flex({
        direction: 'column',
        gap: '6px',
        flexGrow: '1',
        height: 'full',
        paddingX: '10px',
        paddingY: '10px',
        backgroundColor: 'surface.default',
      })}
    >
      <div class={css({ height: '6px', width: '[55%]', borderRadius: 'full', backgroundColor: 'text.default' })}></div>
      <div class={css({ height: '4px', width: '[90%]', borderRadius: 'full', backgroundColor: 'text.muted' })}></div>
      <div
        style:background-color={`color-mix(in srgb, ${selection} 30%, transparent)`}
        class={css({ height: '8px', width: '[70%]', borderRadius: '2px' })}
      ></div>
      <div class={flex({ marginTop: 'auto', gap: '4px' })}>
        <div class={css({ width: '8px', height: '8px', borderRadius: 'full', backgroundColor: 'palette.red' })}></div>
        <div class={css({ width: '8px', height: '8px', borderRadius: 'full', backgroundColor: 'palette.yellow' })}></div>
        <div class={css({ width: '8px', height: '8px', borderRadius: 'full', backgroundColor: 'palette.green' })}></div>
        <div class={css({ width: '8px', height: '8px', borderRadius: 'full', backgroundColor: 'palette.blue' })}></div>
      </div>
    </div>
  </div>
{/snippet}

{#snippet card(
  mode: 'light' | 'dark',
  variant: LightVariant | DarkVariant,
  label: string,
  selection: string,
  selected: boolean,
  onselect: () => void,
)}
  <button
    class={css({
      display: 'flex',
      flexDirection: 'column',
      borderRadius: '10px',
      borderWidth: '1px',
      borderColor: selected ? 'accent.default' : 'border.default',
      cursor: 'pointer',
      transition: 'common',
      overflow: 'hidden',
      _hover: { borderColor: selected ? 'accent.default' : 'border.emphasis' },
    })}
    aria-checked={selected}
    onclick={onselect}
    role="radio"
    type="button"
  >
    <div
      class={css({ width: 'full' })}
      data-theme={mode}
      data-variant-dark={mode === 'dark' ? variant : undefined}
      data-variant-light={mode === 'light' ? variant : undefined}
    >
      {@render preview(selection)}
    </div>
    <div
      class={flex({
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: '6px',
        width: 'full',
        paddingX: '10px',
        paddingY: '8px',
        backgroundColor: 'surface.default',
      })}
    >
      <span
        class={css({
          fontSize: '13px',
          color: selected ? 'text.default' : 'text.muted',
          fontWeight: selected ? 'medium' : 'normal',
          transition: 'common',
        })}
      >
        {label}
      </span>
      {#if selected}
        <Icon style={css.raw({ flexShrink: '0', color: 'accent.default' })} icon={CheckIcon} size={14} />
      {/if}
    </div>
  </button>
{/snippet}

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>테마</h1>
  </div>

  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '4px' })}>라이트 모드</h2>
    <p class={css({ fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]', marginBottom: '20px' })}>
      라이트 모드가 적용되었을 때 표시할 테마를 선택할 수 있어요.
    </p>
    <div
      class={css({ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '12px' })}
      aria-label="라이트 모드 테마"
      role="radiogroup"
    >
      {#each LIGHT_VARIANTS as variant (variant)}
        {@render card(
          'light',
          variant,
          VARIANT_LABELS[`light-${variant}`],
          VARIANT_SELECTION[`light-${variant}`],
          theme.lightVariant === variant,
          () => selectLightVariant(variant),
        )}
      {/each}
    </div>
  </div>

  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '4px' })}>다크 모드</h2>
    <p class={css({ fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]', marginBottom: '20px' })}>
      다크 모드가 적용되었을 때 표시할 테마를 선택할 수 있어요.
    </p>
    <div class={css({ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '12px' })} aria-label="다크 모드 테마" role="radiogroup">
      {#each DARK_VARIANTS as variant (variant)}
        {@render card(
          'dark',
          variant,
          VARIANT_LABELS[`dark-${variant}`],
          VARIANT_SELECTION[`dark-${variant}`],
          theme.darkVariant === variant,
          () => selectDarkVariant(variant),
        )}
      {/each}
    </div>
  </div>
</div>
