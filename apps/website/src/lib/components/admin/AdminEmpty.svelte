<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = {
    text?: string;
  };

  let { text = '데이터가 없습니다' }: Props = $props();
</script>

<div class={css({ paddingY: '48px', fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>{text}</div>
