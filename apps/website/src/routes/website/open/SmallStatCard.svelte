<script lang="ts">
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import Sparkline from './Sparkline.svelte';

  type Props = {
    title: string;
    description: string;
    value: string;
    data: { date: string; value: number }[];
  };

  let { title, description, value, data }: Props = $props();
</script>

<div
  class={css({
    backgroundColor: 'white',
    border: '1px solid',
    borderColor: 'gray.200',
    borderRadius: '[12px]',
    padding: '20px',
    position: 'relative',
  })}
>
  <div class={flex({ justifyContent: 'space-between', alignItems: 'center', marginBottom: '12px' })}>
    <h4 class={css({ fontSize: '13px', fontWeight: 'medium', color: 'gray.600' })}>{title}</h4>
    <Sparkline color="#e5e7eb" {data} height={16} width={48} />
  </div>
  <p class={css({ fontSize: '[28px]', fontWeight: 'semibold', color: 'gray.900', lineHeight: '[1]' })}>
    {value}
  </p>
  <p class={css({ fontSize: '12px', color: 'gray.500', marginTop: '4px' })}>{description}</p>
</div>
