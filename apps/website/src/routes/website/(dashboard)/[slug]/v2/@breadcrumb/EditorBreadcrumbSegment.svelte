<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import FolderIcon from '~icons/lucide/folder';
  import HomeIcon from '~icons/lucide/house';
  import EntityIcon from '../../../@context-menu/EntityIcon.svelte';
  import type { EntityIcon_entity$key } from '$mearie';

  type Props = {
    focused?: boolean;
    isCurrent: boolean;
    item: { kind: 'home' } | { kind: 'entity'; name: string; entity$key: EntityIcon_entity$key };
  };

  let { focused, isCurrent, item }: Props = $props();
</script>

{#if item.kind === 'home'}
  <Icon icon={HomeIcon} size={14} />
{:else}
  <span class={css({ display: 'contents', color: 'text.default' })}>
    <EntityIcon
      style={focused === false
        ? css.raw({
            color: {
              base: '[color-mix(in oklab, currentColor 73%, token(colors.surface.default))]',
              _dark: '[color-mix(in oklab, currentColor 82%, token(colors.surface.default))]',
            },
          })
        : undefined}
      entity$key={item.entity$key}
      fallback={isCurrent ? undefined : FolderIcon}
      size={14}
    />
  </span>
{/if}
<span>{item.kind === 'home' ? '홈' : item.name}</span>
{#if !isCurrent}
  <span class={css({ display: 'grid', placeItems: 'center', color: 'text.hint' })} aria-hidden="true">
    <Icon icon={ChevronRightIcon} size={14} />
  </span>
{/if}
