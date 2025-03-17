<script lang="ts">
  import * as PortOne from '@portone/browser-sdk/v2';
  import Editor from '$assets/graphics/editor.svg?component';
  import Glitters from '$assets/graphics/glitters.svg';
  import Star from '$assets/icons/star.svg?component';
  import Logo from '$assets/logos/logo.svg?component';
  import { Dialog } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';

  let open = $state(false);
  let confirmOpen = $state(false);

  let email = $state('');
  let emailChecked = $state(false);
  let termsChecked = $state(false);

  const handleSubmit = async () => {
    const resp = await PortOne.requestPayment({
      storeId: 'store-e1e69136-38bb-42dd-b226-3c78e03c1ff1',
      channelKey: 'channel-key-6403de1b-8d90-4813-bb8e-0b9a1c094f6b',
      paymentId: 'asdf', // TODO
      orderName: '글리터 사전 등록',
      totalAmount: 100,
      currency: 'CURRENCY_KRW',
      payMethod: 'CARD',
    });

    // TODO
    console.log(resp);
  };
</script>

<div
  style={`background-image: url("${Glitters}"), linear-gradient(180deg, #15124C 0%, #7C77D6 111.89%)`}
  class={css({ position: 'relative' })}
>
  <div
    class={css({
      position: 'absolute',
      zIndex: '0',
      bottom: '[-0.2px]',
      left: '0',
      right: '0',
      width: 'full',
      height: '180px',
      borderColor: 'transparent',
      backgroundColor: '[#FFFDF8]',
      clipPath: 'polygon(0 0, 100% 100%, 0 100%)',
    })}
  ></div>

  <div class={flex({ direction: 'column', align: 'center', position: 'relative', zIndex: '1', paddingTop: '37px', width: 'full' })}>
    <Logo class={css({ height: '32px', color: 'white' })} />

    <div class={css({ marginTop: '51px', marginBottom: '40px', color: 'white', textAlign: 'center' })}>
      <h1 class={css({ marginBottom: '30px', fontFamily: '[IBMPlexSansKR]', fontSize: '[52px]', fontWeight: '[700]' })}>
        창작자가 기다려온
        <br />
        글쓰기 앱을 만나보세요
      </h1>

      <p class={css({ fontFamily: '[LINESeedKR]', fontSize: '18px' })}>
        몰입해서 쓰고, 유연하게 공유하고, 깔끔하게 정리하는 스마트한 에디터, 글리터.
        <br />
        지금 얼리버드 한정 특별가로 미리 등록하세요.
      </p>
    </div>

    <form
      class={flex({
        align: 'center',
        gap: '16px',
        marginBottom: '30px',
        borderWidth: '1px',
        borderColor: '[#d5d5d5]',
        borderRadius: '6px',
        padding: '12px',
        backgroundColor: 'white',
        width: 'full',
        maxWidth: '425px',
      })}
      onsubmit={() => (open = true)}
    >
      <input
        class={css({ flexGrow: '1', fontSize: '14px', fontWeight: '[700]' })}
        placeholder="이메일을 입력해주세요"
        type="text"
        bind:value={email}
      />

      <button
        class={css({
          borderRadius: '4px',
          paddingX: '12px',
          paddingY: '8px',
          fontSize: '14px',
          fontWeight: '[500]',
          color: 'white',
          backgroundColor: '[#494682]',
        })}
        type="submit"
      >
        사전 등록하기
      </button>
    </form>

    <Editor class={css({ marginBottom: '-140px', width: '900px' })} />
  </div>
</div>

<Dialog onsubmit={handleSubmit} bind:open>
  <form class={flex({ direction: 'column', gap: '20px' })} method="dialog">
    <Logo class={css({ height: '24px' })} />

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
        placeholder="글리터에 가장 기대하는 기능이 있다면 적어주세요"
        rows="3"
      ></textarea>
    </div>

    <div class={flex({ align: 'center', gap: '3px' })}>
      <input id="confirmEmail" required type="checkbox" bind:checked={emailChecked} />
      <label class={css({ fontSize: '14px', cursor: 'pointer' })} for="confirmEmail">
        연락받을 이메일 주소가 정확한지 확인해주세요
        <span class={css({ color: '[#ACB2B9]' })}>(필수)</span>
      </label>
    </div>

    <div class={flex({ align: 'center', gap: '3px' })}>
      <input id="confirmTerms" required type="checkbox" bind:checked={termsChecked} />
      <label class={css({ fontSize: '14px', cursor: 'pointer' })} for="confirmTerms">
        <a class={css({ textDecoration: 'underline', textUnderlineOffset: '2px' })} href="https://glitter.rdbl.io/legal/terms">이용약관</a>
        및
        <a class={css({ textDecoration: 'underline', textUnderlineOffset: '2px' })} href="https://glitter.rdbl.io/legal/privacy">
          개인정보 처리방침
        </a>
        동의
        <span class={css({ color: '[#ACB2B9]' })}>(필수)</span>
      </label>
    </div>

    <button
      class={css({
        borderRadius: '8px',
        paddingX: '20px',
        paddingY: '11px',
        fontSize: '12px',
        color: 'white',
        backgroundColor: '[#4A2DA0]',
      })}
      disabled={email.length === 0 || !termsChecked || !termsChecked}
      type="submit"
    >
      4,900원 결제하고 사전 등록하기
    </button>
  </form>

  <p class={css({ marginTop: '20px', fontSize: '10px', color: '[#ACB2B9]' })}>
    정확한 수요 파악을 위해 선입금을 받고 있습니다. 결제하신 금액은 서비스 출시시 자동으로 1개월의 이용 기간으로 전환되며, 출시가 취소될
    경우 전액 환불됩니다.
  </p>
</Dialog>

<Dialog bind:open={confirmOpen}>
  <form>
    글리터 사전 등록이 완료되었어요!

    <button
      class={css({
        borderRadius: '8px',
        paddingX: '20px',
        paddingY: '11px',
        fontSize: '12px',
        color: 'white',
        backgroundColor: '[#4A2DA0]',
      })}
      disabled={email.length === 0 || !termsChecked || !termsChecked}
      type="submit"
    >
      감사합니다. 오픈일에 만나요!
    </button>
  </form>
</Dialog>
