<script lang="ts">
  import * as PortOne from '@portone/browser-sdk/v2';
  import mixpanel from 'mixpanel-browser';
  import Star from '$assets/icons/star.svg?component';
  import Logo from '$assets/logos/logo.svg?component';
  import { env } from '$env/dynamic/public';
  import { graphql } from '$graphql';
  import { Dialog } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';

  const createPreorderPayment = graphql(`
    mutation PaymentModal_CreatePreorderPayment_Mutation($input: CreatePreorderPaymentInput!) {
      createPreorderPayment(input: $input) {
        id
      }
    }
  `);

  const finalizePreorderPayment = graphql(`
    mutation PaymentModal_FinalizePreorderPayment_Mutation($input: FinalizePreorderPaymentInput!) {
      finalizePreorderPayment(input: $input) {
        id
      }
    }
  `);

  type Props = {
    open: boolean;
    email: string;
  };

  let { open = $bindable(false), email = $bindable('') }: Props = $props();

  let confirmOpen = $state(false);

  let feature = $state('');
  let emailChecked = $state(false);
  let termsChecked = $state(false);

  const handleSubmit = async () => {
    const payment = await createPreorderPayment({
      email,
    });

    const resp = await PortOne.requestPayment({
      storeId: 'store-e1e69136-38bb-42dd-b226-3c78e03c1ff1',
      channelKey: 'channel-key-6403de1b-8d90-4813-bb8e-0b9a1c094f6b',
      paymentId: payment.id,
      orderName: '타이피 사전 등록',
      totalAmount: 4900,
      currency: 'CURRENCY_KRW',
      payMethod: 'CARD',
      redirectUrl: `${env.PUBLIC_API_URL}/payment/redirect`,
      customer: {
        email,
      },
      customData: {
        email,
        wish: feature,
      },
    });

    if (resp?.code === undefined) {
      await finalizePreorderPayment({
        email,
        paymentId: payment.id,
        wish: feature,
      });

      mixpanel.track('payment_success', { email });
      confirmOpen = true;
    } else {
      alert('결제에 실패했어요. 다시 시도해주세요.');
    }
  };
</script>

<Dialog onsubmit={handleSubmit} bind:open>
  <form class={flex({ direction: 'column', gap: '20px' })} method="dialog">
    <Logo class={css({ marginBottom: '20px', height: '24px' })} />

    <div class={flex({ direction: 'column' })}>
      <label class={css({ display: 'flex', alignItems: 'flex-start', gap: '2px', marginBottom: '8px', fontSize: '14px' })} for="email">
        이메일

        <Star class={css({ color: '[#E30000]', size: '12px' })} />
      </label>

      <input
        id="email"
        class={css({
          borderWidth: '1px',
          borderColor: 'line.secondary',
          borderRadius: '8px',
          paddingX: '12px',
          paddingY: '8px',
          textStyle: '14m',
        })}
        placeholder="이메일을 입력해주세요"
        required
        type="email"
        bind:value={email}
      />
    </div>

    <div class={flex({ direction: 'column' })}>
      <label class={css({ marginBottom: '8px', fontSize: '14px' })} for="feature">가장 기대하는 기능</label>
      <textarea
        id="feature"
        class={css({
          borderWidth: '1px',
          borderColor: 'line.secondary',
          borderRadius: '8px',
          paddingX: '12px',
          paddingY: '8px',
          textStyle: '14m',
          resize: 'none',
        })}
        placeholder="타이피에 가장 기대하는 기능이 있다면 적어주세요"
        rows="3"
        bind:value={feature}
      ></textarea>
    </div>

    <div class={flex({ direction: 'column', gap: '8px' })}>
      <div class={flex({ align: 'center', gap: '8px' })}>
        <input id="confirmEmail" required type="checkbox" bind:checked={emailChecked} />
        <label class={css({ fontSize: '14px', cursor: 'pointer' })} for="confirmEmail">
          연락받을 이메일 주소가 정확한지 확인해주세요
          <span class={css({ color: '[#ACB2B9]' })}>(필수)</span>
        </label>
      </div>

      <div class={flex({ align: 'center', gap: '8px' })}>
        <input id="confirmTerms" required type="checkbox" bind:checked={termsChecked} />
        <label class={css({ fontSize: '14px', cursor: 'pointer' })} for="confirmTerms">
          <a
            class={css({ textDecoration: 'underline', textUnderlineOffset: '2px' })}
            href="https://typie.rdbl.io/legal/terms"
            rel="noopener noreferrer"
            target="_blank"
          >
            이용약관
          </a>
          및
          <a
            class={css({ textDecoration: 'underline', textUnderlineOffset: '2px' })}
            href="https://typie.rdbl.io/legal/privacy"
            rel="noopener noreferrer"
            target="_blank"
          >
            개인정보 처리방침
          </a>
          동의
          <span class={css({ color: '[#ACB2B9]' })}>(필수)</span>
        </label>
      </div>
    </div>

    <button
      class={css({
        borderRadius: '8px',
        paddingX: '20px',
        paddingY: '12px',
        fontSize: '12px',
        color: 'white',
        backgroundColor: '[#4A2DA0]',
      })}
      type="submit"
    >
      4,900원 결제하고 사전 등록하기
    </button>
  </form>

  <p class={css({ marginTop: '20px', fontSize: '10px', color: '[#ACB2B9]' })}>
    · 정확한 수요 파악을 위해 선입금을 받고 있습니다.
    <br />
    · 결제하신 금액은 서비스 출시시 자동으로 1개월의 이용 기간으로 전환되며, 출시가 취소될 경우 전액 환불됩니다.
    <br />
    · 결제 취소가 필요할 경우 고객센터로 문의 부탁드립니다.
  </p>
</Dialog>

<Dialog bind:open={confirmOpen}>
  <form class={flex({ direction: 'column', align: 'center', gap: '20px', width: 'full' })} method="dialog">
    타이피 사전 등록이 완료되었어요!

    <button
      class={css({
        borderRadius: '8px',
        paddingX: '20px',
        paddingY: '12px',
        fontSize: '12px',
        color: 'white',
        backgroundColor: '[#4A2DA0]',
        width: 'full',
      })}
      type="submit"
    >
      감사합니다. 오픈일에 만나요!
    </button>
  </form>
</Dialog>
