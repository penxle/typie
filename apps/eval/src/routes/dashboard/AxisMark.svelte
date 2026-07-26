<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';

  // 축을 늘 같은 자리·같은 너비로 세운다. 아니오인 칸만 채워지므로, 목록을 세로로 훑으면
  // 어느 축에서 무너지는지가 글을 읽기 전에 무늬로 드러난다.
  type Props = { slots: { label: string; failed: boolean }[] };
  const { slots }: Props = $props();

  const slotClass = css({
    width: '30px',
    paddingY: '1px',
    borderRadius: '3px',
    fontSize: '11px',
    lineHeight: '[1.4]',
    textAlign: 'center',
    fontVariantNumeric: 'tabular-nums',
  });
  const failedClass = css({ backgroundColor: 'accent.danger.default', color: 'text.bright', fontWeight: 'bold' });
  const passedClass = css({ color: 'text.disabled' });
</script>

<span class={flex({ align: 'center', gap: '2px', flexShrink: '0' })}>
  {#each slots as slot (slot.label)}
    <span class={slot.failed ? `${slotClass} ${failedClass}` : `${slotClass} ${passedClass}`}>{slot.label}</span>
  {/each}
</span>
