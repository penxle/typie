<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { onMount } from 'svelte';
  import LinkIcon from '~icons/lucide/link';
  import RubyIcon from '~icons/typie/ruby';

  type Props = {
    kind: 'link' | 'ruby';
    initialValue: string;
    onsubmit: (value: string) => void;
    oncancel?: () => void;
  };

  let { kind, initialValue, onsubmit, oncancel }: Props = $props();

  const existing = initialValue.length > 0;

  let value = $state(initialValue);
  let input = $state<HTMLInputElement>();

  const empty = $derived(value.trim().length === 0);

  const submit = () => {
    if (empty) return;
    onsubmit(kind === 'link' ? value.trim() : value);
  };

  onMount(() => {
    const timer = setTimeout(() => {
      input?.focus({ preventScroll: true });
      if (existing) input?.select();
    }, 0);
    return () => clearTimeout(timer);
  });
</script>

<div class={flex({ alignItems: 'center', gap: '8px', height: '36px', paddingLeft: '12px', paddingRight: '4px' })}>
  <Icon style={css.raw({ flexShrink: '0', color: 'text.hint' })} icon={kind === 'link' ? LinkIcon : RubyIcon} size={14} />

  <input
    bind:this={input}
    class={css({
      flexGrow: '1',
      width: '208px',
      minWidth: '0',
      height: 'full',
      fontSize: '13px',
      fontWeight: 'medium',
      color: 'text.default',
      backgroundColor: 'transparent',
      border: 'none',
      outline: 'none',
      _placeholder: { color: 'text.hint', fontWeight: 'normal' },
    })}
    aria-label={kind === 'link' ? '링크' : '루비'}
    onkeydown={(e) => {
      if (e.isComposing) return;
      if (e.key === 'Enter') {
        e.preventDefault();
        submit();
      } else if (oncancel && e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        oncancel();
      }
    }}
    placeholder={kind === 'link' ? 'https://...' : '텍스트 위에 들어갈 문구'}
    type={kind === 'link' ? 'url' : 'text'}
    bind:value
  />

  <button
    class={css({
      flexShrink: '0',
      height: '28px',
      paddingX: '10px',
      borderRadius: '6px',
      fontSize: '12px',
      fontWeight: 'medium',
      color: 'text.default',
      backgroundColor: 'surface.inset',
      transition: 'common',
      _enabled: { _hover: { backgroundColor: 'surface.active' } },
      _disabled: { opacity: '40' },
    })}
    disabled={empty}
    onclick={submit}
    onpointerdown={(e) => e.preventDefault()}
    type="button"
  >
    {existing ? '수정' : '삽입'}
  </button>
</div>
