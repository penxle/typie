<script lang="ts">
  import Glitters from '$assets/graphics/glitters.svg';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
</script>

<div
  style={`background-image: url("${Glitters}")`}
  class={flex({ direction: 'column', align: 'center', paddingTop: '37px', width: 'full', backgroundColor: '[#4A2DA0]' })}
>
  <div class={css({ marginTop: '56px', marginBottom: '72px', color: 'white', textAlign: 'center' })}>
    <h1 class={css({ marginBottom: '30px', fontFamily: '[LINESeedKR]', fontSize: '[32px]', fontWeight: '[700]' })}>
      창작의 결을 그대로 살리는 에디터
    </h1>

    <p class={css({ fontSize: '20px' })}>
      몰입해서 쓰고, 유연하게 공유하고,
      <br />
      깔끔하게 정리하는 스마트 에디터, 글리터의 첫 이용자가 되어보세요!
    </p>
  </div>

  <form
    class={flex({
      align: 'center',
      gap: '16px',
      marginBottom: '90px',
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
        backgroundColor: '[#4A2DA0]',
      })}
      type="submit"
    >
      사전등록하기
    </button>
  </form>
</div>
