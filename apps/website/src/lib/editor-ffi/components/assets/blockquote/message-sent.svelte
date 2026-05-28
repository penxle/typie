<div class="message-sent-option"></div>

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
    right: -0.3456rem;
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
</style>
