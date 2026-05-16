<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '../editor.svelte';

  const { editor } = getEditorContext();
</script>

<div
  class={css({
    position: 'absolute',
    top: '0',
    left: '0',
    backgroundColor: 'text.default',
    animation: 'blink 1s step-end infinite',
    pointerEvents: 'none',
  })}
  {@attach (el) => {
    if (editor) {
      $effect(() => {
        if (editor.cursor && editor.focused) {
          const { width, height } = editor.cursor.caret;
          el.style.width = `${width}px`;
          el.style.height = `${height}px`;
          el.style.visibility = 'visible';
          el.getAnimations().forEach((a) => (a.currentTime = 0));
        } else {
          el.style.visibility = 'hidden';
        }
      });

      const off = editor.on('state_changed', (_, { fields }) => {
        if (fields.includes('cursor')) {
          el.getAnimations().forEach((a) => (a.currentTime = 0));
        }
      });

      return () => {
        off();
      };
    }
  }}
></div>

<style>
  @keyframes -global-blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0;
    }
  }
</style>
