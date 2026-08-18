<script lang="ts">
  // cspell:ignore ondismiss onrestart

  import { css } from '@typie/styled-system/css';

  type Props = { version: string; onrestart: () => void; ondismiss: () => void };
  let { version, onrestart, ondismiss }: Props = $props();
</script>

<div
  style:-webkit-app-region="no-drag"
  class={css({
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    paddingX: '10px',
    marginY: '6px',
    borderRadius: '6px',
    backgroundColor: 'accent.brand.subtle',
    fontSize: '12px',
  })}
>
  <span>새 버전 {version} 준비됨</span>
  <button class={css({ fontWeight: 'semibold', textDecoration: 'underline' })} onclick={onrestart} type="button">지금 재시작</button>
  <button class={css({ color: 'text.faint' })} onclick={ondismiss} type="button">나중에</button>
</div>
