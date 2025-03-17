<script lang="ts">
  import Timer from '$assets/graphics/timer.svg?component';
  import Tracker from '$assets/graphics/tracker.png';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
</script>

<div class={flex({ justify: 'center', paddingTop: '80px', paddingBottom: '60px', backgroundColor: '[#2D2B56]' })}>
  <div class={css({ width: 'full', maxWidth: '900px' })}>
    <div class={css({ textAlign: 'center', color: 'white' })}>
      <h2 class={css({ fontFamily: '[LINESeedKR]', fontSize: '[42px]', fontWeight: '[700]' })}>몰입을 돕는 최적의 환경 세팅</h2>

      <p class={css({ marginTop: '30px', fontWeight: '[500]' })}>
        집중 모드와 타이머, 목표치 설정 등으로 방해없이
        <br />
        오직 글쓰기에 몰입할 수 있는 환경을 만들 수 있어요
      </p>
    </div>

    <div class={flex({ align: 'center', gap: '30px', marginTop: '60px', textAlign: 'center', height: '318px' })}>
      <div
        class={css({
          flex: 'none',
          borderRadius: '8px',
          paddingX: '30px',
          paddingY: '24px',
          backgroundColor: '[#F3F1F7]',
          width: '460px',
          height: 'full',
        })}
      >
        <p class={css({ '& > b': { fontWeight: '[600]' } })}>
          <b>작품의 목표 글자 수</b>
          와
          <br />
          <b>하루의 목표 글자 수</b>
          를 입력하는 트래커 기능
        </p>

        <img class={css({ marginTop: '20px' })} alt="트래커" src={Tracker} />
      </div>

      <div class={css({ borderRadius: '8px', paddingX: '30px', paddingY: '24px', backgroundColor: '[#F3F1F7]', size: 'full' })}>
        <p class={css({ '& > b': { fontWeight: '[600]' } })}>
          <b>창작에 몰입</b>
          하는 환경을 조성하는
          <br />
          <b>집중 타이머 기능</b>
        </p>

        <Timer class={css({ size: 'full' })} />
      </div>
    </div>
  </div>
</div>
