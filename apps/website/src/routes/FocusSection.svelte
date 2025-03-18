<script lang="ts">
  import Circle from '$assets/graphics/circle.svg?component';
  import Decoration from '$assets/graphics/decoration2.svg?component';
  import Timer from '$assets/graphics/timer.svg?component';
  import Tracker from '$assets/graphics/tracker.png';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
</script>

<div
  class={flex({
    justify: 'center',
    marginTop: '100px',
    marginBottom: { base: '120px', md: '180px' },
    paddingX: '20px',
  })}
>
  <div class={css({ position: 'relative', zIndex: '1', width: 'full', maxWidth: '900px' })}>
    <div
      class={css({
        position: 'absolute',
        left: '0',
        background: '[linear-gradient(355deg, rgba(192, 145, 225, 0.40) 3.63%, rgba(84, 75, 246, 0.40) 70.24%)]',
        filter: 'auto',
        blur: '[100px]',
        transform: 'rotate(90deg)',
        width: '430px',
        height: '610px',
      })}
    ></div>
    <div
      class={css({
        position: 'absolute',
        right: '0',
        background: '[linear-gradient(355deg, rgba(192, 145, 225, 0.40) 3.63%, rgba(84, 75, 246, 0.40) 70.24%)]',
        filter: 'auto',
        blur: '[90px]',
        borderRadius: 'full',
        size: '560px',
        mdDown: { bottom: '0' },
      })}
    ></div>

    <div class={css({ position: 'relative', zIndex: '1', display: 'flex', flexDirection: 'column', alignItems: 'center', width: 'full' })}>
      <div class={flex({ direction: 'column', align: 'center', textAlign: 'center' })}>
        <div class={css({ position: 'relative', width: 'fit' })}>
          <h2 class={css({ fontFamily: '[LINESeedKR]', fontSize: { base: '28px', md: '[42px]' }, fontWeight: '[700]' })}>
            몰입을 돕는 최적의 환경
          </h2>
          <Circle
            class={css({
              position: 'absolute',
              top: '1/2',
              translate: 'auto',
              translateY: '-1/2',
              left: '-26px',
              color: '[#F7DE9C]',
              height: { base: '72px', md: '94px' },
            })}
          />
        </div>

        <p class={css({ marginTop: { base: '16px', md: '30px' }, fontWeight: '[500]' })}>
          집중 모드와 타이머, 목표치 설정 등으로 방해없이
          <br />
          오직 글쓰기에 몰입할 수 있는 환경을 만들 수 있어요.
        </p>
      </div>

      <div
        class={flex({
          align: 'center',
          gap: { base: '16px', md: '30px' },
          position: 'relative',
          marginTop: '60px',
          textAlign: 'center',
          mdDown: { flexDirection: 'column' },
          md: { width: 'full' },
        })}
      >
        <Decoration class={css({ position: 'absolute', top: '-56px', right: '-64px', color: '[#B1D8CB]', width: '100px' })} />
        <Decoration
          class={css({
            position: 'absolute',
            bottom: '-44px',
            left: '-82px',
            transform: 'rotate(-157deg)',
            color: '[#B1ACE2]',
            width: '100px',
          })}
        />
        <div
          class={css({
            display: 'flex',
            flexDirection: 'column',
            borderRadius: '8px',
            paddingX: { base: '20px', md: '30px' },
            paddingY: { base: '16px', md: '24px' },
            backgroundColor: '[#000000]',
            color: 'white',
            width: 'full',
            maxWidth: '460px',
            height: { base: '300px', md: '318px' },
          })}
        >
          <!-- prettier-ignore -->
          <p class={css({ '& > b': { fontWeight: '[600]' } })}>
            <b>작품의 목표 글자 수</b>와
            <br />
            <b>하루의 목표 글자 수</b>를 입력하는 트래커 기능
          </p>

          <img class={css({ marginTop: 'auto' })} alt="트래커" src={Tracker} />
        </div>

        <div
          class={css({
            borderRadius: '8px',
            paddingX: { base: '20px', md: '30px' },
            paddingY: { base: '16px', md: '24px' },
            backgroundColor: '[#000000]',
            color: 'white',
            height: { base: '300px', md: '318px' },
            width: 'full',
            maxWidth: { base: '460px', md: '410px' },
          })}
        >
          <!-- prettier-ignore -->
          <p class={css({ '& > b': { fontWeight: '[600]' } })}>
            <b>창작에 몰입</b>하는 환경을 조성하는
            <br />
            <b>집중 타이머 기능</b>
          </p>

          <Timer class={css({ height: 'full' })} />
        </div>
      </div>
    </div>
  </div>
</div>
