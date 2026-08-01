<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';

  type Props = {
    start: string;
    end: string;
  };

  let { start = $bindable(), end = $bindable() }: Props = $props();

  const inputStyle = css({
    borderWidth: '1px',
    borderColor: 'transparent',
    borderRadius: '8px',
    minHeight: '32px',
    backgroundColor: 'surface.muted',
    paddingX: '8px',
    paddingY: '5px',
    fontSize: '13px',
    color: 'text.default',
    _focus: { borderColor: 'border.brand', outline: 'none' },
  });
</script>

<div class={flex({ alignItems: 'center', gap: '6px' })}>
  <input class={inputStyle} type="date" bind:value={start} />
  <span class={css({ fontSize: '13px', color: 'text.disabled' })}>–</span>
  <input class={inputStyle} type="date" bind:value={end} />
</div>
