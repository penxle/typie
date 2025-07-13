<script lang="ts">
  import { onMount } from 'svelte';
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import MiniDashboardLight from './MiniDashboardLight.svelte';

  let mounted = $state(false);

  onMount(() => {
    mounted = true;
  });
</script>

<div class={css({ width: '[100dvw]', minHeight: '[100dvh]', backgroundColor: 'white' })}>
  <div class={center({ flexDirection: 'column', width: 'full', height: '[90vh]', backgroundColor: 'dark.gray.900' })}>
    <div class={css({ fontSize: '[48px]', fontWeight: 'extrabold', color: 'dark.gray.50', textAlign: 'center', fontFamily: 'Paperlogy' })}>
      온전히 생각을 담아내는
      <br />
      나만의 글쓰기 공간
    </div>

    <div
      class={css({
        fontSize: '[20px]',
        fontWeight: 'medium',
        color: 'dark.gray.50',
        textAlign: 'center',
        fontFamily: 'Pretendard',
        marginTop: '24px',
      })}
    >
      쓰고, 공유하고, 정리하기까지.
      <br />
      하나의 에디터 안에서.
    </div>

    <div
      class={css({
        marginTop: '36px',
        paddingX: '48px',
        paddingY: '12px',
        borderRadius: 'full',
        backgroundColor: 'dark.gray.50',
        color: 'dark.gray.950',
        fontSize: '16px',
        fontWeight: 'bold',
      })}
    >
      시작하기
    </div>
  </div>
  <div class={css({ backgroundColor: 'dark.gray.900' })}>
    <div class={css({ borderTopRadius: 'full', width: 'full', height: '50px', backgroundColor: 'white' })}></div>
  </div>
  <div class={css({ position: 'relative', width: 'full', height: '[400dvh]' })} data-element="mini-dashboard-container">
    <div
      class={css({
        position: 'sticky',
        top: '0',
        left: '0',
        height: '[100dvh]',
        backgroundColor: 'white',
        overflow: 'hidden',
      })}
      data-element="mini-dashboard-wrapper"
    >
      {#if mounted}
        <div class={css({ position: 'relative', size: 'full', pointerEvents: 'none', fontFamily: 'SUIT' })}>
          <MiniDashboardLight />
        </div>
      {/if}
    </div>
  </div>

  <div class={css({ width: 'full', backgroundColor: 'dark.gray.700' })}>
    <div class={css({ width: 'full', height: '50px', backgroundColor: 'white', borderTopRadius: 'full' })}></div>
  </div>
  <div class={css({ width: 'full', height: '[100dvh]', backgroundColor: 'white' })}></div>
</div>
