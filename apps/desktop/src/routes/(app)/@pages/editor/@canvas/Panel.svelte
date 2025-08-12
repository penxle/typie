<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { token } from '@typie/styled-system/tokens';
  import { values } from '@typie/ui/canvas';
  import { getThemeContext } from '@typie/ui/context';
  import PanelButton from './PanelButton.svelte';
  import type { Canvas, Shapes } from '@typie/ui/canvas';

  type Props = {
    canvas: Canvas;
  };

  let { canvas }: Props = $props();

  const node = $derived(canvas.state.selections.length === 1 ? canvas.state.selections[0] : null);
  const type = $derived(node?.current.className as Shapes);

  type Section = 'backgroundColor' | 'backgroundStyle' | 'roughness' | 'borderRadius' | 'fontSize' | 'fontFamily';
  const sections: Record<Shapes, Section[]> = {
    TypedArrow: ['roughness'],
    TypedBrush: ['roughness'],
    TypedEllipse: ['backgroundColor', 'backgroundStyle', 'roughness', 'fontSize', 'fontFamily'],
    TypedLine: ['roughness'],
    TypedRect: ['backgroundColor', 'backgroundStyle', 'roughness', 'borderRadius', 'fontSize', 'fontFamily'],
    TypedStickyNote: ['backgroundColor'],
  };

  const effectiveSections = $derived(type ? sections[type] : []);

  const setAttribute = (property: string, value: unknown) => {
    node?.current.setAttrs({ [property]: value });
  };

  const theme = getThemeContext();
</script>

{#if effectiveSections.length > 0}
  <div
    class={flex({
      position: 'absolute',
      top: '1/2',
      right: '20px',
      flexDirection: 'column',
      gap: '24px',
      borderWidth: '1px',
      borderRadius: '12px',
      padding: '12px',
      width: '240px',
      backgroundColor: 'surface.default',
      boxShadow: 'small',
      translate: 'auto',
      translateY: '-1/2',
      zIndex: 'panel',
    })}
  >
    {#if effectiveSections.includes('backgroundColor')}
      <div class={css({ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: '8px' })}>
        <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })}>배경색</div>

        <div class={flex({ gap: '4px' })}>
          {#each values.backgroundColor as { label, value, color, darkColor } (value)}
            <button
              style:background-color={theme.effective === 'dark' ? darkColor : color}
              style:outline-color={value === 'white' ? token('colors.border.default') : color}
              class={center({
                borderWidth: '1px',
                borderRadius: '4px',
                outlineWidth: node?.current.attrs.backgroundColor === value ? '2px' : '0',
                outlineOffset: '1px',
                size: '20px',
                position: 'relative',
              })}
              aria-label={label}
              onclick={() => setAttribute('backgroundColor', value)}
              type="button"
            ></button>
          {/each}
        </div>
      </div>
    {/if}

    {#if effectiveSections.includes('backgroundStyle')}
      <div class={css({ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: '8px' })}>
        <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })}>배경 채우기</div>

        <div class={flex({ gap: '4px' })}>
          <PanelButton
            active={node?.current.attrs.backgroundStyle === 'solid'}
            label="칠하기"
            onclick={() => setAttribute('backgroundStyle', 'solid')}
          >
            <div class={css({ borderRadius: '4px', size: '16px', backgroundColor: 'current' })}></div>
          </PanelButton>

          <PanelButton
            active={node?.current.attrs.backgroundStyle === 'hachure'}
            label="빗금 칠하기"
            onclick={() => setAttribute('backgroundStyle', 'hachure')}
          >
            <svg class={css({ size: '16px' })} fill="none" viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
              <defs>
                <clipPath id="roundedRect">
                  <rect height="15" rx="3.5" width="15" x="0.5" y="0.5" />
                </clipPath>
              </defs>
              <rect height="15" rx="3.5" stroke="currentColor" width="15" x="0.5" y="0.5" />
              <g clip-path="url(#roundedRect)">
                <path d="M0 5L5 0M0 10L10 0M0 15L15 0M5 15L15 5M10 15L15 10" stroke="currentColor" stroke-width="0.8" />
              </g>
            </svg>
          </PanelButton>

          <PanelButton
            active={node?.current.attrs.backgroundStyle === 'none'}
            label="칠하지 않기"
            onclick={() => setAttribute('backgroundStyle', 'none')}
          >
            <svg class={css({ size: '16px' })} fill="none" viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
              <rect height="15" rx="3.5" stroke="currentColor" stroke-dasharray="2 2" width="15" x="0.5" y="0.5" />
            </svg>
          </PanelButton>
        </div>
      </div>
    {/if}

    {#if effectiveSections.includes('roughness')}
      <div class={css({ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: '8px' })}>
        <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })}>스타일</div>

        <div class={flex({ gap: '4px' })}>
          <PanelButton
            active={node?.current.attrs.roughness === 'rough'}
            label="손으로 그린 듯한 스타일"
            onclick={() => setAttribute('roughness', 'rough')}
          >
            <svg
              class={css({ size: '20px' })}
              fill="none"
              stroke="currentColor"
              stroke-linecap="round"
              stroke-linejoin="round"
              viewBox="0 0 20 20"
            >
              <path
                d="M2.5 11.936c1.737-.879 8.627-5.346 10.42-5.268 1.795.078-.418 5.138.345 5.736.763.598 3.53-1.789 4.235-2.147M2.929 9.788c1.164-.519 5.47-3.28 6.987-3.114 1.519.165 1 3.827 2.121 4.109 1.122.281 3.839-2.016 4.606-2.42"
                stroke-width="1.25"
              ></path>
            </svg>
          </PanelButton>

          <PanelButton
            active={node?.current.attrs.roughness === 'none'}
            label="정확하게 그린 스타일"
            onclick={() => setAttribute('roughness', 'none')}
          >
            <svg
              class={css({ size: '20px' })}
              fill="none"
              stroke="currentColor"
              stroke-linecap="round"
              stroke-linejoin="round"
              viewBox="0 0 20 20"
            >
              <path
                d="M2.5 12.038c1.655-.885 5.9-3.292 8.568-4.354 2.668-1.063.101 2.821 1.32 3.104 1.218.283 5.112-1.814 5.112-1.814"
                stroke-width="1.25"
              ></path>
            </svg>
          </PanelButton>
        </div>
      </div>
    {/if}

    {#if effectiveSections.includes('borderRadius')}
      <div class={css({ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: '8px' })}>
        <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })}>모서리</div>

        <div class={flex({ gap: '4px' })}>
          <PanelButton
            active={node?.current.attrs.borderRadius === 'round'}
            label="둥글게"
            onclick={() => setAttribute('borderRadius', 'round')}
          >
            <div class={css({ borderRadius: '4px', size: '16px', borderWidth: '1px', borderColor: 'current' })}></div>
          </PanelButton>

          <PanelButton
            active={node?.current.attrs.borderRadius === 'none'}
            label="뾰족하게"
            onclick={() => setAttribute('borderRadius', 'none')}
          >
            <div class={css({ size: '16px', borderWidth: '1px', borderColor: 'current' })}></div>
          </PanelButton>
        </div>
      </div>
    {/if}

    {#if effectiveSections.includes('fontSize')}
      <div class={css({ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: '8px' })}>
        <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })}>글씨 크기</div>

        <div class={flex({ gap: '4px' })}>
          <PanelButton active={node?.current.attrs.fontSize === 'small'} label="작게" onclick={() => setAttribute('fontSize', 'small')}>
            <div class={center({ size: '16px', fontSize: '12px', fontWeight: 'medium' })}>S</div>
          </PanelButton>

          <PanelButton active={node?.current.attrs.fontSize === 'medium'} label="보통" onclick={() => setAttribute('fontSize', 'medium')}>
            <div class={center({ size: '16px', fontSize: '16px', fontWeight: 'medium' })}>M</div>
          </PanelButton>

          <PanelButton active={node?.current.attrs.fontSize === 'large'} label="크게" onclick={() => setAttribute('fontSize', 'large')}>
            <div class={center({ size: '16px', fontSize: '20px', fontWeight: 'medium' })}>L</div>
          </PanelButton>
        </div>
      </div>
    {/if}

    {#if effectiveSections.includes('fontFamily')}
      <div class={css({ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: '8px' })}>
        <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })}>글꼴</div>

        <div class={flex({ gap: '4px' })}>
          <PanelButton
            active={node?.current.attrs.fontFamily === 'handwriting'}
            label="손글씨"
            onclick={() => setAttribute('fontFamily', 'handwriting')}
          >
            <div class={center({ size: '16px', fontSize: '16px', fontWeight: 'medium', fontFamily: 'Dovemayo' })}>가</div>
          </PanelButton>

          <PanelButton active={node?.current.attrs.fontFamily === 'sans'} label="정자체" onclick={() => setAttribute('fontFamily', 'sans')}>
            <div class={center({ size: '16px', fontSize: '16px', fontWeight: 'medium', fontFamily: 'Paperlogy' })}>가</div>
          </PanelButton>
        </div>
      </div>
    {/if}
  </div>
{/if}
