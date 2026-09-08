<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ArrowUpIcon from '~icons/lucide/arrow-up';

  type Props = { text: string };

  let { text }: Props = $props();

  const PLACEHOLDER = '메시지를 입력하세요';

  let held = $state('');

  $effect(() => {
    if (text.length > 0) held = text;
  });

  const filled = $derived(text.length > 0);
  const shown = $derived(text || held);

  const composerClass = css({
    position: 'relative',
    display: 'flex',
    alignItems: 'flex-start',
    gap: '12px',
    maxWidth: '[600px]',
    paddingX: '16px',
    paddingY: '13px',
    paddingRight: '[56px]',
    borderWidth: '1px',
    borderColor: 'border.emphasis',
    borderRadius: '12px',
    backgroundColor: 'surface.default',
  });
  const ringClass = css({
    position: 'absolute',
    inset: '[-1px]',
    borderWidth: '1px',
    borderColor: 'accent.default',
    borderRadius: '12px',
    opacity: '0',
    pointerEvents: 'none',
    transition: '[opacity 200ms ease]',
    '&[data-on="true"]': { opacity: '100' },
    _motionReduce: { transition: '[none]' },
  });
  const promptClass = css({
    position: 'relative',
    flexGrow: '1',
    minWidth: '0',
    fontFamily: 'landing',
    fontSize: '15px',
    lineHeight: '[1.5]',
    letterSpacing: '0',
    fontWeight: 'normal',
    color: 'text.default',
    wordBreak: 'keep-all',
    minHeight: '[1.5em]',
    overflow: 'hidden',
  });
  const placeholderClass = css({
    position: 'absolute',
    top: '0',
    left: '0',
    color: 'text.hint',
    pointerEvents: 'none',
    transition: '[opacity 160ms ease]',
    '&[data-hidden="true"]': { opacity: '0' },
    _motionReduce: { transition: '[none]' },
  });
  const textClass = css({
    display: 'block',
    transition: '[opacity 160ms ease]',
    '&[data-sending="true"]': { position: 'absolute', top: '0', left: '0', right: '0', opacity: '0' },
    _motionReduce: { transition: '[none]' },
  });
  const sendClass = flex({
    position: 'absolute',
    right: '10px',
    bottom: '10px',
    align: 'center',
    justify: 'center',
    size: '30px',
    borderRadius: 'full',
    backgroundColor: 'text.default',
    color: 'surface.canvas',
    opacity: '35',
    transition: '[opacity 200ms ease, transform 200ms ease]',
    '&[data-on="true"]': { opacity: '100' },
    '&[data-sending="true"]': { transform: '[scale(0.9)]' },
    _motionReduce: { transition: '[none]' },
  });
</script>

<div class={composerClass}>
  <span class={ringClass} data-on={filled}></span>

  <p class={promptClass}>
    <span class={placeholderClass} data-hidden={filled}>{PLACEHOLDER}</span>
    <span class={textClass} data-sending={!filled}>{shown}</span>
  </p>
  <span class={sendClass} data-on={filled}>
    <Icon icon={ArrowUpIcon} size={16} />
  </span>
</div>
