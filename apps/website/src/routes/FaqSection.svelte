<script lang="ts">
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import Faq from './Faq.svelte';

  const faqs = [
    {
      question: '글리터는 어떤 서비스인가요?',
      answer: '노션, 에버노트, 구글 문서와 유사하나, 작가에게 더 특화된 서비스입니다.',
    },
    {
      question: '이용시 비용이 드나요?',
      answer: '월 4,900원에 작성, 보관, 공유까지 모든 기능을 제한 없이 이용하실 수 있으며, 추가 비용은 발생하지 않습니다.',
    },
    {
      question: '올라온 글을 읽을 때도 결제가 필요한가요?',
      answer: '공유된 글은 결제나 가입 없이 읽으실 수 있습니다.',
    },
    {
      question: '사전예약시 결제가 필요한 이유가 무엇인가요?',
      answer:
        '정확한 수요 파악을 위해 선입금을 받고 있습니다. 결제하신 금액은 서비스 출시시 자동으로 1개월의 이용 기간으로 전환되며, 출시가 취소될 경우 전액 환불됩니다.',
    },
    {
      question: '유료 포스트 발행/후원이 가능한가요?',
      answer: '불가능합니다.',
    },
    {
      question: '앱으로도 제공되나요?',
      answer: '웹 버전 출시 이후 순차적으로 데스크탑 앱 및 모바일 앱이 출시될 예정입니다.',
    },
    {
      question: '또 어떤 기능이 있나요?',
      answer: '아직 공개되지 않은 기능들이 있습니다. 차차 추가될 예정이니 업데이트를 기다려주세요!',
    },
  ];
</script>

<div class={flex({ direction: 'column', align: 'center', paddingTop: '100px', width: 'full', backgroundColor: '[#F9FAFB]' })}>
  <div class={css({ marginBottom: '80px', textAlign: 'center' })}>
    <h1 class={css({ fontFamily: '[LINESeedKR]', fontSize: '[32px]', fontWeight: '[700]' })}>자주 묻는 질문들</h1>
  </div>

  <div class={flex({ direction: 'column', width: 'full', maxWidth: '900px', borderYWidth: '1px', borderYColor: '[#E9E9E9]' })}>
    {#each faqs as { question, answer } (question)}
      <Faq {answer} {question} />
    {/each}
  </div>
</div>
