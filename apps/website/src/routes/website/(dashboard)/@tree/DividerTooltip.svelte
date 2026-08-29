<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import dayjs from 'dayjs';

  type Props = {
    createdAt: string;
  };

  let { createdAt }: Props = $props();
</script>

<div class={flex({ flexDirection: 'column', gap: '4px', minWidth: '140px', opacity: '70' })}>
  <div class={css({ fontWeight: 'semibold' })}>구분선</div>

  <div>생성: {dayjs(createdAt).formatAsDateTime()}</div>
</div>
