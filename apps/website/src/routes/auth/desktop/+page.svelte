<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import Logo from '$assets/logos/logo.svg?component';
</script>

<Helmet title="로그인 완료" />

<div class={flex({ flexDirection: 'column', gap: '24px' })}>
  <div class={flex({ justifyContent: 'flex-start' })}>
    <Logo class={css({ height: '32px' })} />
  </div>

  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
    <h1 class={css({ fontSize: { base: '22px', lg: '24px' }, fontWeight: 'extrabold' })}>로그인이 완료됐어요</h1>
    <div class={css({ fontSize: { base: '13px', lg: '14px' }, color: 'text.muted' })}>타이피 앱으로 돌아가세요. 이 창은 닫아도 돼요.</div>
  </div>
</div>
