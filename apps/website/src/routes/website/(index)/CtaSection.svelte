<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import Glitters from '$assets/graphics/glitters.svg';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import PaymentModal from './PaymentModal.svelte';

  let email = $state('');
  let open = $state(false);
</script>

<div
  style={`background-image: url("${Glitters}")`}
  class={flex({ direction: 'column', align: 'center', paddingY: '120px', paddingX: '20px', width: 'full', backgroundColor: '[#4A2DA0]' })}
>
  <div class={css({ marginBottom: '40px', color: 'white', textAlign: 'center' })}>
    <p class={css({ fontFamily: '[LINESeedKR]', fontSize: '24px', fontWeight: '[700]' })}>
      지금 얼리버드 특별가로
      <br class={css({ hideFrom: 'lg' })} />
      사전 등록하고
      <br />
      타이피의 첫 유저가 되어보세요!
    </p>
  </div>

  <form
    class={flex({
      align: 'center',
      gap: '16px',
      borderWidth: '1px',
      borderColor: '[#d5d5d5]',
      borderRadius: '6px',
      paddingX: { base: '8px', lg: '12px' },
      paddingY: { base: '6px', lg: '8px' },
      backgroundColor: 'white',
      width: 'full',
      maxWidth: '425px',
    })}
    onsubmit={(e) => {
      e.preventDefault();
      open = true;
      mixpanel.track('payment_modal_open', { section: 'cta', email });
    }}
  >
    <input
      class={css({ minWidth: '0', flexGrow: '1', fontSize: '14px', fontWeight: '[700]' })}
      placeholder="이메일을 입력해주세요"
      type="email"
      bind:value={email}
    />

    <button
      class={css({
        flex: 'none',
        borderRadius: '4px',
        paddingX: '12px',
        paddingY: '8px',
        fontSize: '14px',
        fontWeight: '[500]',
        color: 'white',
        backgroundColor: '[#4A2DA0]',
      })}
      type="submit"
    >
      사전 등록하기
    </button>
  </form>
</div>

<PaymentModal bind:open bind:email />
