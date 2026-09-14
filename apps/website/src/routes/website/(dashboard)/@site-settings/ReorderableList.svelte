<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import GripVerticalIcon from '~icons/lucide/grip-vertical';
  import { SettingsDivider } from '$lib/components';
  import { resolveNextFractionalOrderMove } from '$lib/fractional-order';
  import type { Snippet } from 'svelte';

  type Item = { id: string; order: string };
  type Move = { id: string; lowerOrder: string | null; upperOrder: string | null };

  type Props = {
    items: Item[];
    disabled?: boolean;
    onmove: (move: Move) => Promise<unknown>;
    row: Snippet<[string]>;
  };

  let { items, disabled = false, onmove, row }: Props = $props();

  let listElement = $state<HTMLDivElement>();
  let dragging = $state<{ id: string; startY: number; dy: number; dropIndex: number } | null>(null);
  let committing = $state(false);
  let releaseEscape: (() => void) | null = null;

  const ids = $derived(items.map((item) => item.id));
  const locked = $derived(disabled || committing || items.length < 2);

  const dropIndexAt = (clientY: number, draggedId: string): number => {
    if (!listElement) return 0;
    let index = 0;
    for (const element of listElement.querySelectorAll<HTMLElement>('[data-reorder-item]')) {
      if (element.dataset.reorderId === draggedId) continue;
      const { top, bottom } = element.getBoundingClientRect();
      if (clientY > (top + bottom) / 2) index += 1;
    }
    return index;
  };

  const start = (event: PointerEvent, id: string) => {
    if (locked || dragging !== null || event.button !== 0) return;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    dragging = { id, startY: event.clientY, dy: 0, dropIndex: dropIndexAt(event.clientY, id) };
    releaseEscape = pushEscapeHandler(() => {
      cancel();
      return true;
    });
  };

  const move = (event: PointerEvent) => {
    if (!dragging) return;
    dragging.dy = event.clientY - dragging.startY;
    dragging.dropIndex = dropIndexAt(event.clientY, dragging.id);
  };

  const cancel = () => {
    releaseEscape?.();
    releaseEscape = null;
    dragging = null;
  };

  $effect(() => () => cancel());

  const finish = async () => {
    releaseEscape?.();
    releaseEscape = null;

    const current = dragging;
    if (!current) return;
    dragging = null;

    const desired = ids.filter((id) => id !== current.id);
    desired.splice(current.dropIndex, 0, current.id);

    const authoritative = new Map(items.map((item) => [item.id, item.order]));
    const next = resolveNextFractionalOrderMove(authoritative, desired, current.id);
    if (!next) return;

    committing = true;
    try {
      await onmove({ id: next.key, lowerOrder: next.lowerOrder ?? null, upperOrder: next.upperOrder ?? null });
    } finally {
      committing = false;
    }
  };

  const slotOf = (index: number): number | null => {
    if (!dragging) return null;
    const draggedIndex = ids.indexOf(dragging.id);
    if (index === draggedIndex) return null;
    return index > draggedIndex ? index - 1 : index;
  };
</script>

<div bind:this={listElement} class={flex({ flexDirection: 'column' })}>
  {#each ids as id, index (id)}
    {@const isDragged = dragging?.id === id}
    {@const slot = slotOf(index)}

    {#if dragging && slot !== null && slot === dragging.dropIndex}
      <div class={css({ height: '2px', marginX: '20px', borderRadius: 'full', backgroundColor: 'accent.default' })}></div>
    {/if}

    <div
      style:transform={isDragged && dragging ? `translateY(${dragging.dy}px)` : undefined}
      class={css(
        { position: 'relative', display: 'flex', alignItems: 'center', gap: '4px', paddingLeft: '8px' },
        isDragged && { zIndex: '1', backgroundColor: 'surface.default', boxShadow: 'md', opacity: '90' },
      )}
      data-reorder-id={id}
      data-reorder-item
    >
      <button
        class={center({
          flexShrink: '0',
          size: '24px',
          borderRadius: '4px',
          color: 'text.hint',
          cursor: locked ? 'default' : 'grab',
          touchAction: 'none',
          _hover: { color: 'text.default' },
        })}
        aria-label="순서 변경"
        disabled={locked}
        onpointercancel={cancel}
        onpointerdown={(e) => start(e, id)}
        onpointermove={move}
        onpointerup={finish}
        type="button"
      >
        <Icon icon={GripVerticalIcon} size={14} />
      </button>

      <div class={css({ flex: '1', minWidth: '0' })}>
        {@render row(id)}
      </div>
    </div>

    {#if index < ids.length - 1}
      <SettingsDivider />
    {/if}
  {/each}

  {#if dragging && dragging.dropIndex === ids.length - 1}
    <div class={css({ height: '2px', marginX: '20px', borderRadius: 'full', backgroundColor: 'accent.default' })}></div>
  {/if}
</div>
