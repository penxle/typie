<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = {
    value: unknown;
  };

  let { value }: Props = $props();

  const text = $derived.by(() => {
    try {
      return JSON.stringify(value, null, 2) ?? String(value);
    } catch {
      return String(value);
    }
  });
</script>

<pre
  class={css({
    borderRadius: '8px',
    backgroundColor: 'surface.muted',
    padding: '12px',
    fontFamily: 'mono',
    fontSize: '12px',
    color: 'text.subtle',
    overflowX: 'auto',
  })}>{text}</pre>
