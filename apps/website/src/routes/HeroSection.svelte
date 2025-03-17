<script lang="ts">
  import Glitters from '$assets/graphics/glitters.svg?component';
  import Logo from '$assets/logos/logo.svg?component';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
</script>

<div class={css({ position: 'relative', backgroundColor: '[#2D2B56]', height: '732px' })}>
  <Glitters class={css({ size: 'full' })} />

  <div class={flex({ direction: 'column', align: 'center', position: 'absolute', top: '0', left: '0', paddingTop: '37px', width: 'full' })}>
    <Logo class={css({ height: '32px', color: 'white' })} />

    <div class={css({ marginTop: '51px', marginBottom: '40px', color: 'white', textAlign: 'center' })}>
      <h1 class={css({ marginBottom: '30px', fontFamily: '[IBMPlexSansKR]', fontSize: '[52px]', fontWeight: '[700]' })}>
        창작자가 기다려온
        <br />
        글쓰기 앱을 만나보세요
      </h1>

      <p class={css({ fontSize: '18px' })}>몰입해서 쓰고, 유연하게 공유하고, 깔끔하게 정리하는 스마트 에디터, 글리터</p>
    </div>

    <form
      class={flex({
        align: 'center',
        gap: '16px',
        borderWidth: '1px',
        borderColor: '[#d5d5d5]',
        borderRadius: '6px',
        padding: '12px',
        backgroundColor: 'white',
      })}
    >
      <input class={css({ fontSize: '14px', fontWeight: '[700]' })} placeholder="이메일 입력" type="email" />

      <button
        class={css({
          borderRadius: '4px',
          paddingX: '12px',
          paddingY: '8px',
          fontSize: '14px',
          fontWeight: '[500]',
          color: 'white',
          backgroundColor: '[#393a55]',
        })}
        type="submit"
      >
        사전등록하기
      </button>
    </form>

    <div
      class={css({
        marginTop: '30px',
        borderRadius: '4px',
        backgroundColor: '[#d5d5d5]',
        width: '900px',
        height: '400px',
        boxShadow: '[2px 4px 12px 2px rgba(0, 0, 0, 0.08)]',
      })}
    >
      에디터
    </div>
  </div>
</div>
