<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { token } from '@typie/styled-system/tokens';
  import { getEditor } from '$lib/editor/context';

  const editor = getEditor();

  let element = $state<HTMLDivElement>();

  let cursorColor = $derived(token('colors.text.default'));

  $effect(() => {
    if (element) {
      editor.cursor.element = element;
    }
  });
</script>

<div
  bind:this={element}
  style:background-color={cursorColor}
  style="display: none;"
  class={css({
    pointerEvents: 'none',
    position: 'absolute',
    display: 'none',
    width: '1px',
    animation: 'blink 1s step-end infinite',
  })}
></div>

<style>
  @keyframes blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0;
    }
  }
</style>
