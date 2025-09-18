<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';

  type Props = {
    renderAsOption?: boolean;
  };

  let { renderAsOption }: Props = $props();
</script>

<div
  class={cx(
    'message-sent',
    renderAsOption && 'message-sent-option',
    css({
      width: '1/4',
      flexShrink: '0',
    }),
  )}
></div>

<style>
  .message-sent-option {
    width: 80%;
    height: 80%;
    border-radius: 1.15rem;
    background-color: #248bf5;
    line-height: 1.25;
    padding: 0.5rem 0.875rem;
    position: relative;
    word-wrap: break-word;
  }

  .message-sent-option::before {
    content: '';
    position: absolute;
    background-color: inherit;
    right: -0.35rem;
    bottom: 0;
    width: 1rem;
    height: 1rem;
    transform: scaleX(-1);
    mask-image: url('./message-tail.svg');
    mask-size: contain;
    mask-repeat: no-repeat;
    mask-position: center;
    -webkit-mask-image: url('./message-tail.svg');
    -webkit-mask-size: contain;
    -webkit-mask-repeat: no-repeat;
    -webkit-mask-position: center;
  }

  :global([data-type='message-sent']) {
    justify-content: flex-end;
    gap: 0;
  }

  .message-sent + :global([data-node-view-content-editable]) {
    border-radius: 1.15rem;
    line-height: 1.25;
    padding: 0.5rem 0.875rem;
    position: relative;
    word-wrap: break-word;
  }

  .message-sent + :global([data-node-view-content-editable])::before,
  .message-sent + :global([data-node-view-content-editable])::after {
    bottom: -0.1rem;
    content: '';
    height: 1rem;
    position: absolute;
  }

  .message-sent + :global([data-node-view-content-editable]) {
    background-color: #248bf5;
    color: #fff;
  }

  .message-sent + :global([data-node-view-content-editable])::before {
    background-color: inherit;
    right: -0.35rem;
    bottom: 0;
    width: 1rem;
    height: 1rem;
    transform: scaleX(-1);
    mask-image: url('./message-tail.svg');
    mask-size: contain;
    mask-repeat: no-repeat;
    mask-position: center;
    -webkit-mask-image: url('./message-tail.svg');
    -webkit-mask-size: contain;
    -webkit-mask-repeat: no-repeat;
    -webkit-mask-position: center;
  }

  .message-sent + :global([data-node-view-content-editable]) {
    max-width: fit-content;
  }

  .message-sent + :global([data-node-view-content-editable] p) {
    width: fit-content;
  }
</style>
