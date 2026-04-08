<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { Icon, Submenu } from '@typie/ui/components';
  import CheckIcon from '~icons/lucide/check';
  import PaletteIcon from '~icons/lucide/palette';
  import { graphql } from '$mearie';
  import { entityIconColors, entityIcons, getEntityIconColor } from './entity-icons';

  type Props = {
    entityId: string;
    icon: string;
    iconColor: string;
  };

  let { entityId, icon, iconColor }: Props = $props();

  const [updateEntityIcon] = createMutation(
    graphql(`
      mutation EntityIconPicker_UpdateEntityIcon_Mutation($input: UpdateEntityIconInput!) {
        updateEntityIcon(input: $input) {
          id
          icon
          iconColor
        }
      }
    `),
  );

  const handleIconSelect = async (name: string) => {
    await updateEntityIcon(
      { input: { entityId, icon: name, iconColor } },
      { metadata: { cache: { optimisticResponse: { updateEntityIcon: { id: entityId, icon: name, iconColor } } } } },
    );
  };

  const handleColorSelect = async (color: string) => {
    await updateEntityIcon(
      { input: { entityId, icon, iconColor: color } },
      { metadata: { cache: { optimisticResponse: { updateEntityIcon: { id: entityId, icon, iconColor: color } } } } },
    );
  };

  const currentIconColor = $derived(getEntityIconColor(iconColor));
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
        onclick={() => handleColorSelect(c.value)}
        type="button"
      >
        {#if c.value === iconColor}
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
        onclick={() => handleIconSelect(entry.name)}
        type="button"
      >
        <span style:color={currentIconColor} style:transition="color 200ms ease" style:transition-delay="{wave * 20}ms">
          <Icon icon={entry.icon} size={14} />
        </span>
        {#if entry.name === icon}
          <div
            style:background-color={currentIconColor}
            class={css({ position: 'absolute', bottom: '0', width: '3px', height: '3px', borderRadius: 'full' })}
          ></div>
        {/if}
      </button>
    {/each}
  </div>
</Submenu>
