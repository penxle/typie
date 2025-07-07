<script lang="ts">
  import CircleIcon from '~icons/lucide/circle';
  import HandIcon from '~icons/lucide/hand';
  import MousePointer2Icon from '~icons/lucide/mouse-pointer-2';
  import PenIcon from '~icons/lucide/pen';
  import SlashIcon from '~icons/lucide/slash';
  import SquareIcon from '~icons/lucide/square';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import { Icon } from '$lib/components';
  import { center, flex } from '$styled-system/patterns';
  import type { Component } from 'svelte';
  import type { Tool } from '../lib/types';

  type Props = {
    tool: Tool;
  };

  let { tool = $bindable() }: Props = $props();

  const tools: { id: Tool; name: string; icon: Component }[] = [
    { id: 'pan', name: '이동', icon: HandIcon },
    { id: 'select', name: '선택', icon: MousePointer2Icon },
    { id: 'brush', name: '펜', icon: PenIcon },
    { id: 'rectangle', name: '사각형', icon: SquareIcon },
    { id: 'ellipse', name: '원', icon: CircleIcon },
    { id: 'line', name: '직선', icon: SlashIcon },
    { id: 'stickynote', name: '스티커', icon: StickyNoteIcon },
  ];
</script>

<div
  class={flex({
    alignItems: 'center',
    gap: '2px',
    borderWidth: '1px',
    borderRadius: '12px',
    padding: '8px',
    backgroundColor: 'surface.default',
    boxShadow: 'large',
  })}
>
  {#each tools as t (t.id)}
    <button
      class={center({
        borderRadius: '8px',
        size: '40px',
        color: 'text.muted',
        transition: 'common',
        _hover: {
          color: 'text.default',
          backgroundColor: 'interactive.hover',
        },
        _pressed: {
          color: 'text.brand',
          backgroundColor: 'accent.brand.subtle',
        },
      })}
      aria-pressed={tool === t.id}
      onclick={() => (tool = t.id)}
      title={t.name}
      type="button"
    >
      <Icon icon={t.icon} size={20} />
    </button>
  {/each}
</div>
