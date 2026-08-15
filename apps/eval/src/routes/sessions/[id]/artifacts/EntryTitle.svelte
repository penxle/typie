<script lang="ts">
  import { css } from '@typie/styled-system/css';

  // 항목의 리드가 이름일 때 — 14px medium. 식별자가 곧 이름인 항목(기준표 특질 id)은 mono로 같은 무게.
  type Props = { text: string; mono?: boolean };
  const { text, mono = false }: Props = $props();
</script>

<p
  class={css(
    { fontSize: '14px', fontWeight: 'medium', lineHeight: '[1.6]', color: 'text.default', wordBreak: 'keep-all', overflowWrap: 'anywhere' },
    mono && { fontFamily: 'mono', letterSpacing: '0' },
  )}
>
  {text}
</p>
