<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';

  type Props = {
    renderAsOption?: boolean;
  };

  let { renderAsOption }: Props = $props();
</script>

<div
  class={cx(
    'message-received',
    renderAsOption && 'message-received-option',
    css({
      width: '1/4',
      flexShrink: '0',
    }),
  )}
></div>

<style>
  :root[data-theme='light'] {
    --message-from-them-background-color: #e5e5ea;
  }

  :root[data-theme='dark'] {
    --message-from-them-background-color: #37373b;
  }

  .message-received-option {
    width: 80%;
    height: 80%;
    border-radius: 1.15rem;
    background-color: var(--message-from-them-background-color);
    line-height: 1.25;
    padding: 0.5rem 0.875rem;
    position: relative;
    word-wrap: break-word;
  }

  .message-received-option::before {
    content: '';
    position: absolute;
    background-color: inherit;
    left: -0.35rem;
    bottom: 0;
    width: 1rem;
    height: 1rem;
    mask-image: url('./message-tail.svg');
    mask-size: contain;
    mask-repeat: no-repeat;
    mask-position: center;
    -webkit-mask-image: url('./message-tail.svg');
    -webkit-mask-size: contain;
    -webkit-mask-repeat: no-repeat;
    -webkit-mask-position: center;
  }

  :global([data-type='message-received']) {
    flex-flow: row-reverse;
    justify-content: flex-end;
    gap: 0;
  }

  .message-received + :global([data-node-view-content-editable]) {
    border-radius: 1.15rem;
    line-height: 1.25;
    padding: 0.5rem 0.875rem;
    position: relative;
    word-wrap: break-word;
  }

  .message-received + :global([data-node-view-content-editable])::before,
  .message-received + :global([data-node-view-content-editable])::after {
    bottom: -0.1rem;
    content: '';
    height: 1rem;
    position: absolute;
  }

  .message-received + :global([data-node-view-content-editable]) {
    background-color: var(--message-from-them-background-color);
  }

  .message-received + :global([data-node-view-content-editable])::before {
    background-color: inherit;
    left: -0.35rem;
    bottom: 0;
    width: 1rem;
    height: 1rem;
    mask-image: url('./message-tail.svg');
    mask-size: contain;
    mask-repeat: no-repeat;
    mask-position: center;
    -webkit-mask-image: url('./message-tail.svg');
    -webkit-mask-size: contain;
    -webkit-mask-repeat: no-repeat;
    -webkit-mask-position: center;
  }

  .message-received + :global([data-node-view-content-editable]) {
    max-width: fit-content;
  }

  .message-received + :global([data-node-view-content-editable] p) {
    width: fit-content;
  }
</style>
