<script lang="ts">
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import CircleIcon from '~icons/lucide/circle';
  import HandIcon from '~icons/lucide/hand';
  import MinusIcon from '~icons/lucide/minus';
  import MousePointer2Icon from '~icons/lucide/mouse-pointer-2';
  import PencilIcon from '~icons/lucide/pencil';
  import SquareIcon from '~icons/lucide/square';
  import StickyNoteIcon from '~icons/lucide/sticky-note';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import ToolbarButton from './ToolbarButton.svelte';
  import type { Canvas } from './lib/canvas.svelte';

  type Props = {
    canvas: Canvas;
  };

  let { canvas }: Props = $props();
</script>

<div
  class={flex({
    position: 'absolute',
    bottom: '20px',
    left: '1/2',
    alignItems: 'center',
    gap: '8px',
    borderWidth: '1px',
    borderRadius: '12px',
    padding: '8px',
    backgroundColor: 'surface.default',
    boxShadow: 'small',
    translate: 'auto',
    translateX: '-1/2',
    zIndex: '10',
  })}
>
  <ToolbarButton
    active={canvas.state.tool === 'select'}
    icon={MousePointer2Icon}
    label="선택"
    onclick={() => (canvas.state.tool = 'select')}
    shortcut="V"
  />
  <ToolbarButton
    active={canvas.state.tool === 'pan'}
    icon={HandIcon}
    label="이동"
    onclick={() => (canvas.state.tool = 'pan')}
    shortcut="M"
  />

  <div class={css({ width: '1px', height: '32px', backgroundColor: 'border.default' })}></div>

  <ToolbarButton
    active={canvas.state.tool === 'rectangle'}
    icon={SquareIcon}
    label="사각형"
    onclick={() => (canvas.state.tool = 'rectangle')}
    shortcut="R"
  />
  <ToolbarButton
    active={canvas.state.tool === 'ellipse'}
    icon={CircleIcon}
    label="원"
    onclick={() => (canvas.state.tool = 'ellipse')}
    shortcut="O"
  />
  <ToolbarButton
    active={canvas.state.tool === 'line'}
    icon={MinusIcon}
    label="직선"
    onclick={() => (canvas.state.tool = 'line')}
    shortcut="L"
  />
  <ToolbarButton
    active={canvas.state.tool === 'arrow'}
    icon={ArrowRightIcon}
    label="화살표"
    onclick={() => (canvas.state.tool = 'arrow')}
    shortcut="A"
  />

  <div class={css({ width: '1px', height: '32px', backgroundColor: 'border.default' })}></div>

  <ToolbarButton
    active={canvas.state.tool === 'brush'}
    icon={PencilIcon}
    label="펜"
    onclick={() => (canvas.state.tool = 'brush')}
    shortcut="P"
  />
  <ToolbarButton
    active={canvas.state.tool === 'stickynote'}
    icon={StickyNoteIcon}
    label="메모"
    onclick={() => (canvas.state.tool = 'stickynote')}
    shortcut="T"
  />
</div>
