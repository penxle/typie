<script lang="ts">
  import { onDestroy } from 'svelte';
  import AlarmClockIcon from '~icons/lucide/alarm-clock';
  import MinusIcon from '~icons/lucide/minus';
  import PlusIcon from '~icons/lucide/plus';
  import XIcon from '~icons/lucide/x';
  import { createFloatingActions } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';

  const app = getAppContext();

  let open = $state(false);
  let focusDuration = $state(app.preference.current.focusDuration ?? 30);
  let restDuration = $state(app.preference.current.restDuration ?? 10);
  let timerInterval: ReturnType<typeof setInterval> | null = $state(null);
  let showRestModal = $state(false);

  const durationStep = 5;

  $effect(() => {
    if (app.timerState.current.status !== 'init' && !app.timerState.current.paused && !timerInterval) {
      startTimer();
    }
  });

  $effect(() => {
    if (app.timerState.current.status === 'rest' && !app.timerState.current.keepFocus) {
      showRestModal = true;
    }
  });

  const { anchor, floating } = createFloatingActions({
    placement: 'top-start',
    offset: 6,
    onClickOutside: () => {
      open = false;
    },
  });

  const formatTime = (seconds: number): string => {
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;

    return `${minutes.toString().padStart(2, '0')}:${remainingSeconds.toString().padStart(2, '0')}`;
  };

  const startTimer = () => {
    if (timerInterval) return;

    app.timerState.current.paused = false;

    timerInterval = setInterval(() => {
      if (app.timerState.current.currentTime <= 0) {
        if (app.timerState.current.status === 'focus') {
          app.timerState.current.status = 'rest';
          app.timerState.current.currentTime = restDuration * 60;
        } else {
          app.timerState.current.keepFocus = false;
          app.timerState.current.status = 'focus';
          app.timerState.current.currentTime = focusDuration * 60;
        }
      } else {
        app.timerState.current.currentTime--;
      }
    }, 1000);
  };

  const resetTimer = (e: MouseEvent) => {
    e.stopPropagation();

    if (timerInterval) {
      clearInterval(timerInterval);
      timerInterval = null;
    }

    app.timerState.current.paused = false;
    app.timerState.current.status = 'init';
    app.timerState.current.currentTime = 0;
    app.timerState.current.keepFocus = false;
  };

  onDestroy(() => {
    if (timerInterval) {
      clearInterval(timerInterval);
    }
  });
</script>

<button
  class={css({ display: 'flex', alignItems: 'center', gap: '6px' }, app.timerState.current.status === 'focus' && { color: 'brand.500' })}
  aria-expanded={open}
  onclick={() => (open = !open)}
  type="button"
  use:anchor
>
  <Icon icon={AlarmClockIcon} size={14} />
  <div class={css({ fontSize: '14px' })}>
    {app.timerState.current.status === 'init' ? '타이머' : formatTime(app.timerState.current.currentTime)}
  </div>
</button>

{#if open}
  <div
    class={css({
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      gap: '12px',
      borderWidth: '1px',
      borderColor: 'gray.200',
      borderRadius: '12px',
      padding: '16px',
      backgroundColor: 'white',
      minWidth: '240px',
      boxShadow: 'xlarge',
      overflowY: 'auto',
      zIndex: '50',
    })}
    role="menu"
    use:floating
  >
    <div class={flex({ align: 'center', justify: 'space-between', width: 'full' })}>
      <p>타이머</p>

      <button
        class={css({ borderRadius: '4px', padding: '2px', _hover: { backgroundColor: 'gray.100' } })}
        onclick={() => (open = false)}
        type="button"
      >
        <Icon icon={XIcon} />
      </button>
    </div>

    {#if app.timerState.current.status === 'init'}
      <div class={flex({ direction: 'column', gap: '8px' })}>
        <div class={css({ fontSize: '14px', fontWeight: 'medium', textAlign: 'center' })}>작업 시간</div>
        <div class={flex({ alignItems: 'center', gap: '12px' })}>
          <button
            class={css({
              padding: '4px',
              borderRadius: '6px',
              backgroundColor: 'gray.100',
              _hover: { backgroundColor: 'gray.200' },
            })}
            onclick={() => (focusDuration = Math.max(5, focusDuration - durationStep))}
            type="button"
          >
            <Icon icon={MinusIcon} size={16} />
          </button>
          <div class={css({ fontSize: '16px', fontWeight: 'medium' })}>{focusDuration}:00</div>
          <button
            class={css({
              padding: '4px',
              borderRadius: '6px',
              backgroundColor: 'gray.100',
              _hover: { backgroundColor: 'gray.200' },
            })}
            onclick={() => (focusDuration = focusDuration + durationStep)}
            type="button"
          >
            <Icon icon={PlusIcon} size={16} />
          </button>
        </div>
      </div>

      <div class={flex({ direction: 'column', gap: '8px' })}>
        <div class={css({ fontSize: '14px', fontWeight: 'medium', textAlign: 'center' })}>휴식 시간</div>
        <div class={flex({ alignItems: 'center', gap: '12px' })}>
          <button
            class={css({
              padding: '4px',
              borderRadius: '6px',
              backgroundColor: 'gray.100',
              _hover: { backgroundColor: 'gray.200' },
            })}
            onclick={() => (restDuration = Math.max(5, restDuration - durationStep))}
            type="button"
          >
            <Icon icon={MinusIcon} size={16} />
          </button>
          <div class={css({ fontSize: '16px', fontWeight: 'medium' })}>{restDuration}:00</div>
          <button
            class={css({
              padding: '4px',
              borderRadius: '6px',
              backgroundColor: 'gray.100',
              _hover: { backgroundColor: 'gray.200' },
            })}
            onclick={() => (restDuration = restDuration + durationStep)}
            type="button"
          >
            <Icon icon={PlusIcon} size={16} />
          </button>
        </div>
      </div>

      <div class={flex({ gap: '8px', marginTop: '10px', width: 'full' })}>
        <button
          class={css({
            flex: '1',
            padding: '8px',
            borderRadius: '8px',
            backgroundColor: 'brand.500',
            color: 'white',
            _hover: { backgroundColor: 'brand.600' },
          })}
          onclick={() => {
            app.timerState.current.status = 'focus';
            app.timerState.current.currentTime = focusDuration * 60;
            app.preference.current.focusDuration = focusDuration;
            app.preference.current.restDuration = restDuration;

            startTimer();
          }}
          type="button"
        >
          시작
        </button>
        <button
          class={css({
            flex: '1',
            padding: '8px',
            borderRadius: '8px',
            backgroundColor: 'gray.100',
            _hover: { backgroundColor: 'gray.200' },
          })}
          onclick={(e) => resetTimer(e)}
          type="button"
        >
          초기화
        </button>
      </div>
    {:else}
      <div class={flex({ direction: 'column', gap: '8px', textAlign: 'center' })}>
        <div class={css({ fontSize: '14px', fontWeight: 'medium' })}>남은 시간</div>
        <div class={css({ fontSize: '24px', fontWeight: 'bold' })}>
          {formatTime(app.timerState.current.currentTime)}
        </div>
        <div class={css({ fontSize: '14px', color: 'gray.500' })}>
          {app.timerState.current.status === 'focus' ? '작업 중' : '휴식 중'}
        </div>
      </div>
      <div class={flex({ gap: '8px', marginTop: '10px', width: 'full' })}>
        <button
          class={css({
            flex: '1',
            padding: '8px',
            borderRadius: '8px',
            backgroundColor: 'brand.500',
            color: 'white',
            _hover: { backgroundColor: 'brand.600' },
          })}
          onclick={() => {
            if (timerInterval) {
              clearInterval(timerInterval);
              timerInterval = null;
              app.timerState.current.paused = true;
            } else {
              app.timerState.current.paused = false;
              startTimer();
            }
          }}
          type="button"
        >
          {app.timerState.current.paused ? '재시작' : '일시정지'}
        </button>
        <button
          class={css({
            flex: '1',
            padding: '8px',
            borderRadius: '8px',
            backgroundColor: 'gray.100',
            _hover: { backgroundColor: 'gray.200' },
          })}
          onclick={(e) => resetTimer(e)}
          type="button"
        >
          초기화
        </button>
      </div>
    {/if}
  </div>
{/if}

{#if showRestModal}
  <div
    class={css({
      position: 'fixed',
      inset: '0',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      backgroundColor: 'gray.900/24',
      zIndex: '50',
    })}
  >
    <div
      class={css({
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: '16px',
        padding: '24px',
        backgroundColor: 'white',
        borderRadius: '12px',
        minWidth: '320px',
        boxShadow: 'xlarge',
      })}
    >
      <div class={css({ fontSize: '20px', fontWeight: 'bold' })}>휴식 시간</div>
      <div class={css({ fontSize: '14px', color: 'gray.600', textAlign: 'center' })}>
        지금은 휴식 시간입니다. 잠시 일을 멈추고 휴식을 취하세요.
      </div>
      <div class={css({ fontSize: '24px', fontWeight: 'bold', color: 'brand.500' })}>
        {formatTime(app.timerState.current.currentTime)}
      </div>
      <div class={flex({ gap: '8px', width: 'full' })}>
        <button
          class={css({
            flex: '1',
            padding: '12px',
            borderRadius: '8px',
            backgroundColor: 'brand.500',
            color: 'white',
            _hover: { backgroundColor: 'brand.600' },
          })}
          onclick={() => {
            app.timerState.current.keepFocus = true;
            showRestModal = false;
          }}
          type="button"
        >
          그래도 작업하기
        </button>
      </div>
    </div>
  </div>
{/if}
