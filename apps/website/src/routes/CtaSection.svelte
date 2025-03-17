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
    <p class={css({ fontFamily: '[LINESeedKR]', fontSize: '24px', fontWeight: '[700]' })}>
      지금 얼리버드 특별가로 사전 등록하고
      <br />
      글리터의 첫 유저가 되어보세요!
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
    <input class={css({ flexGrow: '1', fontSize: '14px', fontWeight: '[700]' })} placeholder="이메일을 입력해주세요" type="email" />

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
      사전 등록하기
    </button>
  </form>
</div>
