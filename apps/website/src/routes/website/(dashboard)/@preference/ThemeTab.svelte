<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import CheckIcon from '~icons/lucide/check';
  import type { DarkVariant, LightVariant } from '@typie/ui/context';

  const theme = getThemeContext();

  type Variant<T extends string> = {
    id: T;
    label: string;
    previewColor: string;
  };

  const lightVariants: Variant<LightVariant>[] = [
    { id: 'white', label: '화이트', previewColor: '#ffffff' },
    { id: 'snow', label: '스노우', previewColor: '#dceafa' },
    { id: 'butter', label: '버터', previewColor: '#f8f4d0' },
    { id: 'peach', label: '피치', previewColor: '#f8c8b0' },
    { id: 'rose', label: '로즈', previewColor: '#f8dae8' },
    { id: 'lavender', label: '라벤더', previewColor: '#e0dcf0' },
    { id: 'mint', label: '민트', previewColor: '#d0ece0' },
    { id: 'latte', label: '라떼', previewColor: '#e8e0d4' },
  ];

  const darkVariants: Variant<DarkVariant>[] = [
    { id: 'black', label: '블랙', previewColor: '#0a0a0a' },
    { id: 'charcoal', label: '차콜', previewColor: '#282830' },
    { id: 'graphite', label: '그래파이트', previewColor: '#3a3c44' },
    { id: 'midnight', label: '미드나이트', previewColor: '#242460' },
    { id: 'navy', label: '네이비', previewColor: '#0a1830' },
    { id: 'obsidian', label: '옵시디언', previewColor: '#301848' },
    { id: 'storm', label: '스톰', previewColor: '#1c2830' },
    { id: 'espresso', label: '에스프레소', previewColor: '#381c10' },
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
      theme.overrideTheme = undefined;
    };
  });
</script>

<div class={flex({ direction: 'column', gap: '40px', maxWidth: '640px' })}>
  <div>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>테마</h1>
  </div>

  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '4px' })}>라이트 모드</h2>
    <p class={css({ fontSize: '13px', color: 'text.subtle', lineHeight: '[1.6]', marginBottom: '20px' })}>
      라이트 모드가 적용되었을 때 표시할 테마를 선택할 수 있어요.
    </p>
    <div class={css({ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '12px' })}>
      {#each lightVariants as variant (variant.id)}
        <button
          class={css({
            display: 'flex',
            flexDirection: 'column',
            borderRadius: '10px',
            borderWidth: '1px',
            borderColor: theme.lightVariant === variant.id ? 'accent.brand.default' : 'border.default',
            cursor: 'pointer',
            transition: 'common',
            overflow: 'hidden',
            _hover: { borderColor: theme.lightVariant === variant.id ? 'accent.brand.default' : 'border.subtle' },
          })}
          onclick={() => selectLightVariant(variant.id)}
          type="button"
        >
          <div
            style:background-color={variant.previewColor}
            class={css({
              position: 'relative',
              width: 'full',
              height: '48px',
              borderBottomWidth: '1px',
              borderColor: 'border.subtle',
            })}
          >
            {#if theme.lightVariant === variant.id}
              <div
                class={center({
                  position: 'absolute',
                  top: '6px',
                  right: '6px',
                  width: '18px',
                  height: '18px',
                  borderRadius: 'full',
                  backgroundColor: 'accent.brand.default',
                })}
              >
                <Icon style={css.raw({ color: 'white' })} icon={CheckIcon} size={12} />
              </div>
            {/if}
          </div>
          <div class={css({ paddingX: '10px', paddingY: '8px', backgroundColor: 'surface.default' })}>
            <span
              class={css({
                fontSize: '13px',
                color: theme.lightVariant === variant.id ? 'text.default' : 'text.muted',
                fontWeight: theme.lightVariant === variant.id ? 'medium' : 'normal',
                transition: 'common',
              })}
            >
              {variant.label}
            </span>
          </div>
        </button>
      {/each}
    </div>
  </div>

  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '4px' })}>다크 모드</h2>
    <p class={css({ fontSize: '13px', color: 'text.subtle', lineHeight: '[1.6]', marginBottom: '20px' })}>
      다크 모드가 적용되었을 때 표시할 테마를 선택할 수 있어요.
    </p>
    <div class={css({ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '12px' })}>
      {#each darkVariants as variant (variant.id)}
        <button
          class={css({
            display: 'flex',
            flexDirection: 'column',
            borderRadius: '10px',
            borderWidth: '1px',
            borderColor: theme.darkVariant === variant.id ? 'accent.brand.default' : 'border.default',
            cursor: 'pointer',
            transition: 'common',
            overflow: 'hidden',
            _hover: { borderColor: theme.darkVariant === variant.id ? 'accent.brand.default' : 'border.subtle' },
          })}
          onclick={() => selectDarkVariant(variant.id)}
          type="button"
        >
          <div
            style:background-color={variant.previewColor}
            class={css({
              position: 'relative',
              width: 'full',
              height: '48px',
              borderBottomWidth: '1px',
              borderColor: 'border.subtle',
            })}
          >
            {#if theme.darkVariant === variant.id}
              <div
                class={center({
                  position: 'absolute',
                  top: '6px',
                  right: '6px',
                  width: '18px',
                  height: '18px',
                  borderRadius: 'full',
                  backgroundColor: 'accent.brand.default',
                })}
              >
                <Icon style={css.raw({ color: 'white' })} icon={CheckIcon} size={12} />
              </div>
            {/if}
          </div>
          <div class={css({ paddingX: '10px', paddingY: '8px', backgroundColor: 'surface.default' })}>
            <span
              class={css({
                fontSize: '13px',
                color: theme.darkVariant === variant.id ? 'text.default' : 'text.muted',
                fontWeight: theme.darkVariant === variant.id ? 'medium' : 'normal',
                transition: 'common',
              })}
            >
              {variant.label}
            </span>
          </div>
        </button>
      {/each}
    </div>
  </div>
</div>
