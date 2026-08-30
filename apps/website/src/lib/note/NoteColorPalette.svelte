<script lang="ts">
  import { createNoteColorDrag } from './note-color-drag';
  import { noteColors } from './note-colors';
  import type { NoteColorOption } from './note-colors';

  type Props = {
    selectedColor: string;
    onchange: (color: string) => void;
    colors?: readonly NoteColorOption[];
    label?: string;
    menuItems?: boolean;
    size?: string;
  };

  let { selectedColor, onchange, colors = noteColors, label = '노트 색상', menuItems = false, size = '16px' }: Props = $props();

  const selectColor = (color: string) => {
    if (color === selectedColor) return;
    onchange(color);
  };
</script>

<div
  style:--note-color-size={size}
  class="note-color-palette"
  aria-label={label}
  role={menuItems ? 'presentation' : 'group'}
  use:createNoteColorDrag={{ onchange: selectColor }}
>
  {#each colors as option (option.value)}
    <button
      style:--note-color={option.color}
      class="note-color-button"
      aria-checked={menuItems ? selectedColor === option.value : undefined}
      aria-label={option.label}
      aria-pressed={menuItems ? undefined : selectedColor === option.value}
      data-note-color-value={option.value}
      onclick={() => selectColor(option.value)}
      role={menuItems ? 'menuitemradio' : undefined}
      title={option.label}
      type="button"
    >
      <span>{option.label}</span>
    </button>
  {/each}
</div>

<style>
  .note-color-palette {
    display: flex;
    align-items: center;
    gap: 4px;
    touch-action: none;
  }

  .note-color-button {
    width: var(--note-color-size);
    height: var(--note-color-size);
    padding: 0;
    border: 1.5px solid var(--note-color);
    border-radius: 9999px;
    background: transparent;
    cursor: pointer;
  }

  .note-color-button[aria-checked='true'],
  .note-color-button[aria-pressed='true'] {
    background: var(--note-color);
  }

  .note-color-button > span {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
