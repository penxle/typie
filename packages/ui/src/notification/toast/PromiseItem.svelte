<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import RingSpinner from '../../components/RingSpinner.svelte';
  import Component from '../sonner/Component.svelte';

  type Props = {
    message: string;
    loading?: boolean;
  };

  const { message, loading = false, ...rest }: Props = $props();
</script>

<Component {...rest}>
  <div class={flex({ alignItems: 'center', gap: '10px' })}>
    <div
      class={css({
        display: 'grid',
        gridTemplateColumns: loading ? '1fr' : '0fr',
        opacity: loading ? '100' : '0',
        transition: '[grid-template-columns 200ms, opacity 200ms]',
      })}
    >
      <div class={css({ overflow: 'hidden' })}>
        <RingSpinner style={css.raw({ height: '[1lh]', color: 'text.faint' })} />
      </div>
    </div>
    <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>
      {message}
    </div>
  </div>
</Component>
