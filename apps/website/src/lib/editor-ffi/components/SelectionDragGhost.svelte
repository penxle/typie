<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import ArchiveIcon from '~icons/lucide/archive';
  import ChevronsDownUpIcon from '~icons/lucide/chevrons-down-up';
  import FilePlusIcon from '~icons/lucide/file-plus';
  import FileQuestionIcon from '~icons/lucide/file-question-mark';
  import FileUpIcon from '~icons/lucide/file-up';
  import GalleryVerticalEndIcon from '~icons/lucide/gallery-vertical-end';
  import ImageIcon from '~icons/lucide/image';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import TableIcon from '~icons/lucide/table';
  import TextIcon from '~icons/lucide/type';
  import HorizontalRuleIcon from '~icons/typie/horizontal-rule';
  import type { DragGhost, NodeType } from '@typie/editor-ffi/browser';
  import type { Component } from 'svelte';

  let { ghost }: { ghost: DragGhost } = $props();

  const icons: Partial<Record<NodeType, Component>> = {
    image: ImageIcon,
    file: PaperclipIcon,
    embed: FileUpIcon,
    table: TableIcon,
    callout: GalleryVerticalEndIcon,
    fold: ChevronsDownUpIcon,
    horizontal_rule: HorizontalRuleIcon,
    page_break: FilePlusIcon,
    archived: ArchiveIcon,
    unknown: FileQuestionIcon,
  };
</script>

<div
  class={css({
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'flex-start',
    gap: '4px',
    width: '[max-content]',
    maxWidth: '320px',
    boxSizing: 'border-box',
    paddingX: '8px',
    paddingY: '4px',
    borderRadius: 'full',
    backgroundColor: 'accent.default',
    color: 'surface.default',
    fontFamily: '[{fonts.ui}, sans-serif]',
    fontSize: '14px',
    fontWeight: 'medium',
    lineHeight: '[20px]',
    pointerEvents: 'none',
    userSelect: 'none',
  })}
  aria-hidden="true"
  data-selection-drag-ghost
>
  {#if ghost.text || ghost.blocks.length === 0}
    <div class={css({ display: 'flex', alignItems: 'center', gap: '2px', maxWidth: 'full', minWidth: '0' })}>
      <Icon style={css.raw({ flexShrink: '0' })} icon={icons[ghost.kind] ?? TextIcon} size={14} />
      {#if ghost.text}
        <span class={css({ minWidth: '0', whiteSpace: 'nowrap' })} data-drag-excerpt>{ghost.text}</span>
      {/if}
    </div>
  {/if}
  {#if ghost.blocks.length > 0}
    <div
      style:font-size={ghost.text ? '12px' : undefined}
      class={css({ display: 'flex', flexWrap: 'wrap', alignItems: 'center', columnGap: '8px', rowGap: '2px', maxWidth: 'full' })}
      data-drag-blocks
    >
      {#each ghost.blocks as block (block.kind)}
        <span class={css({ display: 'inline-flex', alignItems: 'center', gap: '2px', whiteSpace: 'nowrap' })} data-drag-kind={block.kind}>
          <Icon icon={icons[block.kind] ?? TextIcon} size={14} />
          {#if block.count > 1}{block.count}{/if}
        </span>
      {/each}
    </div>
  {/if}
</div>
