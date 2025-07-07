<script lang="ts">
  import { cubicInOut } from 'svelte/easing';
  import { fade } from 'svelte/transition';
  import { match } from 'ts-pattern';
  import ArrowUpRightIcon from '~icons/lucide/arrow-up-right';
  import CircleIcon from '~icons/lucide/circle';
  import PaletteIcon from '~icons/lucide/palette';
  import SlashIcon from '~icons/lucide/slash';
  import SquareIcon from '~icons/lucide/square';
  import TypeIcon from '~icons/lucide/type';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex, grid } from '$styled-system/patterns';
  import { TypedArrow, TypedBrush, TypedEllipse, TypedLine, TypedRect, TypedStickyNote } from '../lib/shapes';
  import { values } from '../lib/values';
  import type { Canvas } from '../lib/canvas.svelte';
  import type { Shapes } from '../lib/types';

  type Props = {
    canvas: Canvas;
  };

  let { canvas }: Props = $props();

  const node = $derived(canvas.state.selections.length === 1 ? canvas.state.selections[0] : null);

  const type = $derived.by((): Shapes | null => {
    if (!node) return null;

    if (node.current instanceof TypedArrow) return 'arrow';
    if (node.current instanceof TypedBrush) return 'brush';
    if (node.current instanceof TypedLine) return 'line';
    if (node.current instanceof TypedEllipse) return 'ellipse';
    if (node.current instanceof TypedRect) return 'rectangle';
    if (node.current instanceof TypedStickyNote) return 'stickynote';

    return null;
  });

  type SectionType = 'backgroundColor' | 'backgroundStyle' | 'roughness' | 'borderRadius' | 'fontSize' | 'coordinates';

  const sectionsForType: Record<Shapes, SectionType[]> = {
    arrow: ['roughness', 'coordinates'],
    rectangle: ['backgroundColor', 'backgroundStyle', 'roughness', 'borderRadius', 'coordinates'],
    ellipse: ['backgroundColor', 'backgroundStyle', 'roughness', 'coordinates'],
    line: ['roughness', 'coordinates'],
    stickynote: ['backgroundColor', 'fontSize', 'coordinates'],
    brush: ['coordinates'],
  };

  const visibleSections = $derived(type ? sectionsForType[type] : []);

  function updateNodeProperty(property: string, value: string | number) {
    if (!node?.current) return;

    node?.current.setAttrs({ [property]: value });
    canvas.selection.update();
  }
</script>

{#if node && type}
  <div
    class={css({
      position: 'absolute',
      top: '20px',
      left: '20px',
      width: '280px',
      zIndex: '10',
      borderWidth: '1px',
      borderRadius: '12px',
      backgroundColor: 'surface.default',
      boxShadow: 'large',
      overflow: 'hidden',
    })}
    transition:fade={{ duration: 150, easing: cubicInOut }}
  >
    <div
      class={flex({
        alignItems: 'center',
        gap: '8px',
        paddingX: '16px',
        paddingY: '12px',
        borderBottomWidth: '1px',
        backgroundColor: 'surface.subtle',
      })}
    >
      <Icon
        icon={match(type)
          .with('rectangle', () => SquareIcon)
          .with('ellipse', () => CircleIcon)
          .with('stickynote', () => TypeIcon)
          .with('line', () => SlashIcon)
          .with('arrow', () => ArrowUpRightIcon)
          .otherwise(() => SquareIcon)}
        size={16}
      />
      <span class={css({ fontSize: '14px', fontWeight: 'medium' })}>
        {match(type)
          .with('rectangle', () => '사각형 속성')
          .with('ellipse', () => '원 속성')
          .with('stickynote', () => '스티커 노트 속성')
          .with('line', () => '선 속성')
          .with('arrow', () => '화살표 속성')
          .otherwise(() => '속성')}
      </span>
    </div>

    <div class={css({ padding: '16px' })}>
      <div class={flex({ direction: 'column', gap: '16px' })}>
        {#if visibleSections.includes('backgroundColor')}
          <div>
            <label class={css({ display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '8px' })}>
              <Icon icon={PaletteIcon} size={14} />
              <span class={css({ fontSize: '13px', color: 'text.muted' })}>배경색</span>
            </label>
            <div class={grid({ columns: 4, gap: '8px' })}>
              {#each values.backgroundColor as color (color.value)}
                <button
                  style:background-color={color.hex}
                  class={css({
                    position: 'relative',
                    aspectRatio: '1/1',
                    borderRadius: '8px',
                    borderWidth: '2px',
                    borderColor: node?.current.attrs.backgroundColor === color.value ? 'brand.600' : 'gray.200',
                    transition: 'common',
                    cursor: 'pointer',
                    _hover: {
                      transform: 'scale(1.05)',
                    },
                  })}
                  aria-label={color.label}
                  onclick={() => updateNodeProperty('backgroundColor', color.value)}
                  title={color.label}
                  type="button"
                ></button>
              {/each}
            </div>
          </div>
        {/if}

        {#if visibleSections.includes('backgroundStyle')}
          <div>
            <div class={css({ fontSize: '13px', color: 'text.muted', marginBottom: '8px', display: 'block' })}>배경 스타일</div>
            <div class={flex({ gap: '8px' })}>
              {#each values.backgroundStyle as style (style.value)}
                <button
                  class={css({
                    flex: '1',
                    paddingY: '6px',
                    borderRadius: '6px',
                    borderWidth: '1px',
                    borderColor: node?.current.attrs.backgroundStyle === style.value ? 'brand.600' : 'gray.200',
                    backgroundColor: node?.current.attrs.backgroundStyle === style.value ? 'accent.brand.subtle' : 'surface.default',
                    fontSize: '13px',
                    transition: 'common',
                    cursor: 'pointer',
                    _hover: {
                      backgroundColor: 'interactive.hover',
                    },
                  })}
                  onclick={() => updateNodeProperty('backgroundStyle', style.value)}
                  type="button"
                >
                  {style.label}
                </button>
              {/each}
            </div>
          </div>
        {/if}

        {#if visibleSections.includes('roughness')}
          <div>
            <div class={css({ fontSize: '13px', color: 'text.muted', marginBottom: '8px', display: 'block' })}>선 스타일</div>
            <div class={flex({ gap: '8px' })}>
              {#each values.roughness as option (option.value)}
                <button
                  class={css({
                    flex: '1',
                    paddingY: '6px',
                    borderRadius: '6px',
                    borderWidth: '1px',
                    borderColor: node?.current.attrs.roughness === option.value ? 'brand.600' : 'gray.200',
                    backgroundColor: node?.current.attrs.roughness === option.value ? 'accent.brand.subtle' : 'surface.default',
                    fontSize: '13px',
                    transition: 'common',
                    cursor: 'pointer',
                    _hover: {
                      backgroundColor: 'interactive.hover',
                    },
                  })}
                  onclick={() => updateNodeProperty('roughness', option.value)}
                  type="button"
                >
                  {option.label}
                </button>
              {/each}
            </div>
          </div>
        {/if}

        {#if visibleSections.includes('borderRadius')}
          <div>
            <div class={css({ fontSize: '13px', color: 'text.muted', marginBottom: '8px', display: 'block' })}>모서리</div>
            <div class={flex({ gap: '8px' })}>
              {#each values.borderRadius as option (option.value)}
                <button
                  class={css({
                    flex: '1',
                    paddingY: '6px',
                    borderRadius: '6px',
                    borderWidth: '1px',
                    borderColor: (node?.current as TypedRect).attrs.borderRadius === option.value ? 'brand.600' : 'gray.200',
                    backgroundColor:
                      (node?.current as TypedRect).attrs.borderRadius === option.value ? 'accent.brand.subtle' : 'surface.default',
                    fontSize: '13px',
                    transition: 'common',
                    cursor: 'pointer',
                    _hover: {
                      backgroundColor: 'interactive.hover',
                    },
                  })}
                  onclick={() => updateNodeProperty('borderRadius', option.value)}
                  type="button"
                >
                  {option.label}
                </button>
              {/each}
            </div>
          </div>
        {/if}

        {#if visibleSections.includes('fontSize')}
          <div>
            <label class={css({ fontSize: '13px', color: 'text.muted', marginBottom: '8px', display: 'block' })} for="font-size-select">
              글자 크기
            </label>
            <select
              id="font-size-select"
              class={css({
                width: 'full',
                paddingX: '12px',
                paddingY: '6px',
                borderRadius: '6px',
                borderWidth: '1px',
                borderColor: 'gray.200',
                backgroundColor: 'surface.default',
                fontSize: '13px',
                cursor: 'pointer',
                _hover: {
                  borderColor: 'gray.400',
                },
              })}
              onchange={(e) => updateNodeProperty('fontSize', Number(e.currentTarget.value))}
              value={(node?.current as TypedStickyNote).attrs.fontSize}
            >
              {#each values.fontSize as size (size.value)}
                <option value={size.value}>{size.label}</option>
              {/each}
            </select>
          </div>
        {/if}

        {#if visibleSections.includes('coordinates')}
          <div class={css({ paddingTop: '8px', borderTopWidth: '1px' })}>
            <div class={grid({ columns: 2, gap: '8px' })}>
              <div>
                <div class={css({ fontSize: '11px', color: 'text.muted', display: 'block' })}>X 좌표</div>
                <div class={css({ fontSize: '13px', marginTop: '2px' })}>{Math.round(node?.current.x())}</div>
              </div>
              <div>
                <div class={css({ fontSize: '11px', color: 'text.muted', display: 'block' })}>Y 좌표</div>
                <div class={css({ fontSize: '13px', marginTop: '2px' })}>{Math.round(node?.current.y())}</div>
              </div>
              {#if 'width' in node.current.attrs}
                <div>
                  <div class={css({ fontSize: '11px', color: 'text.muted', display: 'block' })}>너비</div>
                  <div class={css({ fontSize: '13px', marginTop: '2px' })}>{Math.round(node?.current.attrs.width)}</div>
                </div>
                <div>
                  <div class={css({ fontSize: '11px', color: 'text.muted', display: 'block' })}>높이</div>
                  <div class={css({ fontSize: '13px', marginTop: '2px' })}>{Math.round(node?.current.attrs.height)}</div>
                </div>
              {:else if type === 'ellipse'}
                <div>
                  <div class={css({ fontSize: '11px', color: 'text.muted', display: 'block' })}>너비</div>
                  <div class={css({ fontSize: '13px', marginTop: '2px' })}>
                    {Math.round((node?.current as TypedEllipse).attrs.radiusX * 2)}
                  </div>
                </div>
                <div>
                  <div class={css({ fontSize: '11px', color: 'text.muted', display: 'block' })}>높이</div>
                  <div class={css({ fontSize: '13px', marginTop: '2px' })}>
                    {Math.round((node?.current as TypedEllipse).attrs.radiusY * 2)}
                  </div>
                </div>
              {:else if type === 'line'}
                <div>
                  <div class={css({ fontSize: '11px', color: 'text.muted', display: 'block' })}>길이</div>
                  <div class={css({ fontSize: '13px', marginTop: '2px' })}>
                    {Math.round(
                      Math.sqrt(Math.pow((node?.current as TypedLine).attrs.dx, 2) + Math.pow((node?.current as TypedLine).attrs.dy, 2)),
                    )}
                  </div>
                </div>
                <div>
                  <div class={css({ fontSize: '11px', color: 'text.muted', display: 'block' })}>각도</div>
                  <div class={css({ fontSize: '13px', marginTop: '2px' })}>
                    {Math.round(
                      (Math.atan2((node?.current as TypedLine).attrs.dy, (node?.current as TypedLine).attrs.dx) * 180) / Math.PI,
                    )}°
                  </div>
                </div>
              {:else if type === 'arrow'}
                <div>
                  <div class={css({ fontSize: '11px', color: 'text.muted', display: 'block' })}>길이</div>
                  <div class={css({ fontSize: '13px', marginTop: '2px' })}>
                    {Math.round(
                      Math.sqrt(Math.pow((node?.current as TypedArrow).attrs.dx, 2) + Math.pow((node?.current as TypedArrow).attrs.dy, 2)),
                    )}
                  </div>
                </div>
                <div>
                  <div class={css({ fontSize: '11px', color: 'text.muted', display: 'block' })}>각도</div>
                  <div class={css({ fontSize: '13px', marginTop: '2px' })}>
                    {Math.round(
                      (Math.atan2((node?.current as TypedArrow).attrs.dy, (node?.current as TypedArrow).attrs.dx) * 180) / Math.PI,
                    )}°
                  </div>
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
