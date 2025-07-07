<script lang="ts">
  import ArrowUpRightIcon from '~icons/lucide/arrow-up-right';
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
    { id: 'arrow', name: '화살표', icon: ArrowUpRightIcon },
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
    backgroundColor: 'white',
    boxShadow: 'large',
  })}
>
  {#each tools as t (t.id)}
    <button
      class={center({
        borderRadius: '8px',
        size: '40px',
        color: 'gray.600',
        transition: 'common',
        _hover: {
          color: 'gray.900',
          backgroundColor: 'gray.200',
        },
        _pressed: {
          color: 'brand.500',
          backgroundColor: 'brand.100',
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
