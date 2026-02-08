<div class="message-received-option"></div>

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
    left: -0.3456rem;
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
</style>
