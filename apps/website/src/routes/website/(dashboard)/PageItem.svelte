<script lang="ts">
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import PageList from './PageList.svelte';
  import type { Item } from './types';

  type Props = {
    item: Item;
    depth: number;
    onPointerDown: (e: PointerEvent, item: Item) => void;
    registerNode: (node: HTMLElement | undefined, item: Item & { depth: number }) => void;
  };

  let { item, depth, onPointerDown, registerNode }: Props = $props();

  let open = $state(false);
  let itemEl: HTMLElement;

  $effect(() => {
    registerNode(itemEl, { ...item, depth });
  });
</script>

<li
  bind:this={itemEl}
  id={item.id}
  class={cx(item.type === 'folder' ? 'dnd-item-folder' : 'dnd-item-page', css({ userSelect: 'none' }))}
  onpointerdown={(e) => onPointerDown(e, item)}
>
  {#if item.type === 'folder'}
    <details bind:open>
      <summary
        class={cx(
          'dnd-item-body',
          css({
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            paddingX: '8px',
            paddingY: '6px',
            borderRadius: '6px',
            fontSize: '14px',
            cursor: 'pointer',
            listStyleType: 'none',
            fontWeight: 'medium',
            color: 'gray.700',
            _hover: {
              backgroundColor: 'gray.100',
            },
          }),
        )}
      >
        <span class={css({ display: 'flex', alignItems: 'center', width: '16px', height: '16px', color: 'gray.500' })}>
          {#if open}
            <Icon icon={ChevronUpIcon} size={14} />
          {:else}
            <Icon icon={ChevronDownIcon} size={14} />
          {/if}
        </span>
        <span class={css({ display: 'flex', alignItems: 'center', width: '16px', height: '16px', color: 'gray.500' })}>
          <Icon icon={FolderIcon} size={14} />
        </span>
        <span class={css({ fontSize: '14px', lineHeight: '[1.2]' })}>{item.title}</span>
      </summary>

      {#if item.children}
        <PageList depth={depth + 1} items={item.children} parent={item} />
      {/if}
    </details>
  {:else}
    <a
      class={cx(
        'dnd-item-body',
        css({
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          paddingX: '8px',
          paddingY: '6px',
          borderRadius: '6px',
          fontWeight: 'medium',
          fontSize: '14px',
          color: 'gray.600',
          textDecoration: 'none',
          _hover: {
            backgroundColor: 'gray.100',
          },
        }),
      )}
      draggable="false"
      href="/home"
    >
      <span class={css({ display: 'flex', alignItems: 'center', width: '16px', height: '16px', marginLeft: '22px', color: 'gray.500' })}>
        <Icon icon={FileIcon} size={14} />
      </span>
      <span class={css({ fontSize: '14px', lineHeight: '[1.2]' })}>{item.title}</span>
    </a>
  {/if}
</li>
