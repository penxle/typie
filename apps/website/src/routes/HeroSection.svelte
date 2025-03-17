<script lang="ts">
  import Editor from '$assets/graphics/editor.svg?component';
  import Glitters from '$assets/graphics/glitters.svg';
  import Logo from '$assets/logos/logo.svg?component';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
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

      <p class={css({ fontSize: '18px' })}>몰입해서 쓰고, 유연하게 공유하고, 깔끔하게 정리하는 스마트 에디터, 글리터</p>
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
    >
      <input class={css({ flexGrow: '1', fontSize: '14px', fontWeight: '[700]' })} placeholder="이메일 입력" type="email" />

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
        사전등록하기
      </button>
    </form>

    <Editor class={css({ marginBottom: '-140px', width: '900px' })} />
  </div>
</div>
