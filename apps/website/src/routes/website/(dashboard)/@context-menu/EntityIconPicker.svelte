<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { Icon, Submenu } from '@typie/ui/components';
  import CheckIcon from '~icons/lucide/check';
  import PaletteIcon from '~icons/lucide/palette';
  import { entityIconColors, entityIcons, getEntityIconColor } from './entity-icons';

  type Props = {
    icon?: string;
    iconColor?: string;
    onIconSelect: (name: string) => void;
    onColorSelect: (color: string) => void;
  };

  let { icon, iconColor, onIconSelect, onColorSelect }: Props = $props();

  const currentIconColor = $derived(getEntityIconColor(iconColor ?? 'gray'));
</script>

<Submenu icon={PaletteIcon} label="아이콘 변경">
  <div
    class={css({
      display: 'flex',
      justifyContent: 'space-evenly',
      paddingY: '6px',
      borderBottomWidth: '1px',
      borderColor: 'border.default',
    })}
  >
    {#each entityIconColors as c (c.value)}
      <button
        style:background-color={c.color}
        class={center({
          width: '16px',
          height: '16px',
          borderRadius: 'full',
          cursor: 'pointer',
          transition: 'common',
          _hover: { boxShadow: '[0 0 0 2px token(colors.border.strong)]' },
        })}
        aria-label={c.label}
        onclick={() => onColorSelect(c.value)}
        type="button"
      >
        {#if iconColor !== undefined && c.value === iconColor}
          <Icon style={css.raw({ color: 'surface.default' })} icon={CheckIcon} size={10} />
        {/if}
      </button>
    {/each}
  </div>

  <div
    class={css({
      display: 'grid',
      gridTemplateColumns: 'repeat(7, 1fr)',
      gap: '2px',
      paddingX: '6px',
      paddingY: '6px',
      maxHeight: '210px',
      overflowY: 'auto',
      scrollbarWidth: '[thin]',
      scrollbarGutter: 'stable both-edges',
    })}
  >
    {#each entityIcons as entry, i (entry.name)}
      {@const wave = Math.floor(i / 7) + (i % 7)}
      <button
        class={center({
          position: 'relative',
          width: '24px',
          height: '24px',
          borderRadius: '4px',
          cursor: 'pointer',
          transition: 'common',
          _hover: { backgroundColor: 'surface.muted' },
        })}
        onclick={() => onIconSelect(entry.name)}
        type="button"
      >
        <span style:color={currentIconColor} style:transition="color 200ms ease" style:transition-delay="{wave * 20}ms">
          <Icon icon={entry.icon} size={14} />
        </span>
        {#if icon !== undefined && entry.name === icon}
          <div
            style:background-color={currentIconColor}
            class={css({ position: 'absolute', bottom: '0', width: '3px', height: '3px', borderRadius: 'full' })}
          ></div>
        {/if}
      </button>
    {/each}
  </div>
</Submenu>
