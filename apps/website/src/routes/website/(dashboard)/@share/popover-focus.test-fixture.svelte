<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Modal, Popover } from '@typie/ui/components';

  let open = $state(false);
  let textareaEl = $state<HTMLTextAreaElement>();

  $effect(() => {
    if (!open) return;

    const el = textareaEl;
    if (!el) return;

    const frame = requestAnimationFrame(() => el.focus());
    return () => cancelAnimationFrame(frame);
  });
</script>

<Modal open>
  <Popover bind:open>
    {#snippet trigger()}
      <span>미리보기 문구</span>
    {/snippet}

    <textarea bind:this={textareaEl} class={css({ width: '200px', height: '80px' })} aria-label="미리보기 문구"></textarea>
  </Popover>
</Modal>
