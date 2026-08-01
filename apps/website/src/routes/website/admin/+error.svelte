<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { page } from '$app/state';

  const message = $derived(page.status === 404 ? '찾을 수 없습니다' : '불러오지 못했습니다');
</script>

<div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '12px', paddingY: '80px' })}>
  <div class={css({ fontSize: '15px', fontWeight: 'semibold', color: 'text.default' })}>{message}</div>
  <div class={css({ fontSize: '13px', color: 'text.faint' })}>{page.error?.message ?? ''}</div>
  <Button onclick={() => location.reload()} size="sm" variant="secondary">다시 시도</Button>
</div>
