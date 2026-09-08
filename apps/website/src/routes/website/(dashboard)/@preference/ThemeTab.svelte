<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { DARK_VARIANTS, LIGHT_VARIANTS, VARIANT_LABELS, VARIANT_SELECTION } from '@typie/styled-system/presets';
  import { Icon } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import { Dialog } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import CheckIcon from '~icons/lucide/check';
  import MonitorIcon from '~icons/lucide/monitor';
  import MoonIcon from '~icons/lucide/moon';
  import SunIcon from '~icons/lucide/sun';
  import type { DarkVariant, LightVariant, Theme } from '@typie/ui/context';

  const theme = getThemeContext();
  const modes: { value: Theme; label: string; icon: typeof SunIcon }[] = [
    { value: 'light', label: '라이트 모드', icon: SunIcon },
    { value: 'dark', label: '다크 모드', icon: MoonIcon },
    { value: 'auto', label: '시스템 설정', icon: MonitorIcon },
  ];

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
      const previewMode = theme.overrideTheme;
      if (!previewMode) return;

      const savedMode = theme.currentTheme;
      const systemMode = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      if (previewMode === (savedMode === 'auto' ? systemMode : savedMode)) {
        theme.overrideTheme = undefined;
        return;
      }

      const previewLabel = previewMode === 'light' ? '라이트' : '다크';
      const savedLabel = { light: '라이트 모드', dark: '다크 모드', auto: '시스템 설정' }[savedMode];
      const returnTo = `${savedLabel}${savedMode === 'auto' ? '으로' : '로'}`;
      Dialog.confirm({
        title: `${previewLabel} 모드를 유지할까요?`,
        message: `선택한 테마는 저장됐어요. 현재 ${previewLabel} 모드를 유지하거나 ${returnTo} 돌아갈 수 있어요.`,
        actionLabel: `${previewLabel} 모드 유지`,
        cancelLabel: `${returnTo} 돌아가기`,
        actionHandler: () => {
          theme.currentTheme = previewMode;
        },
        onclose: () => {
          theme.overrideTheme = undefined;
        },
      });
    };
  });
</script>

{#snippet modePreview(mode: 'light' | 'dark')}
  <div class={css({ width: 'full' })} data-theme={mode} data-variant-dark={theme.darkVariant} data-variant-light={theme.lightVariant}>
    {@render preview(
      VARIANT_SELECTION[mode === 'light' ? (`light-${theme.lightVariant}` as const) : (`dark-${theme.darkVariant}` as const)],
    )}
  </div>
{/snippet}

{#snippet preview(selection: string)}
  <div class={flex({ height: '80px', width: 'full', borderBottomWidth: '1px', borderColor: 'border.hairline' })} aria-hidden="true">
    <div
      class={flex({
        direction: 'column',
        gap: '7px',
        width: '[28%]',
        flexShrink: '0',
        height: 'full',
        paddingX: '6px',
        paddingY: '10px',
        backgroundColor: 'surface.canvas',
        borderRightWidth: '1px',
        borderColor: 'border.hairline',
      })}
    >
      <div class={css({ height: '3px', width: '[65%]', borderRadius: 'full', backgroundColor: 'text.hint' })}></div>
      <div class={css({ height: '10px', width: 'full', borderRadius: '3px', backgroundColor: 'surface.active' })}></div>
      <div class={css({ height: '3px', width: '[80%]', borderRadius: 'full', backgroundColor: 'text.hint' })}></div>
    </div>
    <div
      class={flex({
        direction: 'column',
        gap: '8px',
        flexGrow: '1',
        minWidth: '0',
        height: 'full',
        padding: '10px',
        backgroundColor: 'surface.default',
      })}
    >
      <div class={flex({ alignItems: 'center', justifyContent: 'space-between', gap: '6px' })}>
        <div class={css({ height: '5px', width: '[50%]', borderRadius: 'full', backgroundColor: 'text.default' })}></div>
        <div class={css({ height: '9px', width: '18px', borderRadius: '3px', backgroundColor: 'accent.default' })}></div>
      </div>
      <div class={css({ height: '3px', width: '[90%]', borderRadius: 'full', backgroundColor: 'text.muted' })}></div>
      <div
        style:background-color={`color-mix(in srgb, ${selection} 30%, transparent)`}
        class={css({ height: '8px', width: '[70%]', borderRadius: '2px' })}
      ></div>
      <div class={flex({ gap: '4px', marginTop: 'auto' })}>
        <div class={css({ size: '8px', borderRadius: 'full', backgroundColor: 'palette.red' })}></div>
        <div class={css({ size: '8px', borderRadius: 'full', backgroundColor: 'palette.yellow' })}></div>
        <div class={css({ size: '8px', borderRadius: 'full', backgroundColor: 'palette.green' })}></div>
        <div class={css({ size: '8px', borderRadius: 'full', backgroundColor: 'palette.blue' })}></div>
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
      minWidth: '0',
      borderRadius: '10px',
      borderWidth: '1px',
      borderColor: selected ? 'accent.default' : 'border.default',
      cursor: 'pointer',
      transition: 'common',
      overflow: 'hidden',
      _hover: { borderColor: selected ? 'accent.default' : 'border.emphasis' },
    })}
    aria-checked={selected}
    aria-label={label}
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

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '720px' })}>
  <div class={flex({ direction: 'column', gap: '20px' })}>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>테마</h1>
    <div
      class={css({ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '12px', width: 'full', maxWidth: '520px' })}
      aria-label="화면 모드"
      role="radiogroup"
    >
      {#each modes as { value, label, icon } (value)}
        {@const selected = theme.currentTheme === value}
        <button
          class={flex({
            direction: 'column',
            minWidth: '0',
            borderRadius: '10px',
            borderWidth: '1px',
            borderColor: selected ? 'accent.default' : 'border.default',
            backgroundColor: 'surface.default',
            cursor: 'pointer',
            transition: 'common',
            overflow: 'hidden',
            _hover: { borderColor: selected ? 'accent.default' : 'border.emphasis' },
          })}
          aria-checked={selected}
          aria-label={label}
          onclick={() => {
            mixpanel.track('switch_theme', { old: theme.currentTheme, new: value, via: 'theme_settings' });
            theme.currentTheme = value;
            theme.overrideTheme = undefined;
          }}
          role="radio"
          type="button"
        >
          <div class={css({ position: 'relative', width: 'full' })} aria-hidden="true">
            {@render modePreview(value === 'dark' ? 'dark' : 'light')}
            {#if value === 'auto'}
              <div class={css({ position: 'absolute', inset: '0', clipPath: '[inset(0 0 0 50%)]' })}>
                {@render modePreview('dark')}
              </div>
            {/if}
          </div>
          <div class={flex({ alignItems: 'center', gap: '6px', width: 'full', paddingX: '10px', paddingY: '8px' })}>
            <Icon style={css.raw({ color: selected ? 'text.default' : 'text.muted' })} {icon} size={14} />
            <span class={css({ fontSize: '13px', fontWeight: selected ? 'medium' : 'normal', color: 'text.default' })}>{label}</span>
            {#if selected}
              <Icon style={css.raw({ marginLeft: 'auto', color: 'accent.default' })} icon={CheckIcon} size={14} />
            {/if}
          </div>
        </button>
      {/each}
    </div>
  </div>

  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '4px' })}>라이트 모드</h2>
    <p class={css({ fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]', marginBottom: '20px' })}>
      라이트 모드에서 사용할 테마를 선택하고 미리 볼 수 있어요.
    </p>
    <div
      class={css({ display: 'grid', gridTemplateColumns: '[repeat(auto-fit, minmax(140px, 1fr))]', gap: '12px' })}
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
      다크 모드에서 사용할 테마를 선택하고 미리 볼 수 있어요.
    </p>
    <div
      class={css({ display: 'grid', gridTemplateColumns: '[repeat(auto-fit, minmax(140px, 1fr))]', gap: '12px' })}
      aria-label="다크 모드 테마"
      role="radiogroup"
    >
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
