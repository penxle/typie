<script lang="ts">
  import Logo from '$assets/logos/logo.svg?component';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
</script>

<div class={flex({ direction: 'column', gap: '20px', padding: '100px', fontSize: '14px', fontWeight: '[400]' })}>
  <div>
    <Logo class={css({ height: '32px' })} />
  </div>
  <div class={flex({ direction: 'column', gap: '30px' })}>
    <div class={css({ fontWeight: '[500]' })}>(주) 펜슬컴퍼니</div>
    <div class={css({ lineHeight: '[1.6]' })}>
      <div>대표 : 배준현</div>
      <div>개인정보관리책임자 : 배준현</div>
      <div>사업자등록번호 : 610-88-03078</div>
      <div>통신판매업 신고번호 : 2023-서울강남-4541</div>
      <div>주소 : 서울특별시 강남구 강남대로100길 14, 6층</div>
      <div>전화 : 02-565-7695</div>
    </div>
  </div>
  <div class={flex({ direction: 'column', gap: '8px' })}>
    <div class={css({ color: '[#A8A8A8]' })}>All rights reserved ⓒ 2025 PENXLE COMPANY</div>
    <div class={css({ color: '[#636363]' })}>
      <a class={css({ fontWeight: '[500]', color: '[#3260E1]' })} href="https://glitter.rdbl.io/legal/privacy">개인정보처리방침</a>
      |
      <a class={css({ color: '[#3260E1]' })} href="https://glitter.rdbl.io/legal/terms">이용약관</a>
    </div>
  </div>
</div>
