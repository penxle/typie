<script lang="ts">
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
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
            height: '40px',
            cursor: 'pointer',
            listStyleType: 'none',
            backgroundColor: 'gray.200',
          }),
        )}
      >
        {#if open}
          <Icon icon={ChevronUpIcon} />
        {:else}
          <Icon icon={ChevronDownIcon} />
        {/if}
        <span>{item.title}</span>
      </summary>

      {#if item.children}
        <PageList depth={depth + 1} items={item.children} parent={item} />
      {/if}
    </details>
  {:else}
    <a class={cx('dnd-item-body', css({ display: 'flex', alignItems: 'center', height: '40px' }))} draggable="false" href="/home">
      {item.title}
    </a>
  {/if}
</li>
