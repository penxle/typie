<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon, SegmentButtons } from '@typie/ui/components';
  import BellIcon from '~icons/lucide/bell';
  import BellOffIcon from '~icons/lucide/bell-off';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import ClockIcon from '~icons/lucide/clock';
  import PauseIcon from '~icons/lucide/pause';
  import PlayIcon from '~icons/lucide/play';
  import RotateCcwIcon from '~icons/lucide/rotate-ccw';
  import TimerIcon from '~icons/lucide/timer';
  import Volume2Icon from '~icons/lucide/volume-2';
  import VolumeOffIcon from '~icons/lucide/volume-x';
  import Widget from '../Widget.svelte';
  import { getWidgetContext } from '../widget-context.svelte';

  type Props = {
    widgetId: string;
    data?: Record<string, unknown>;
  };

  let { widgetId, data = {} }: Props = $props();

  const widgetContext = getWidgetContext();

  type TimerMode = 'timer' | 'stopwatch';

  let mode = $state<TimerMode>((data.mode as TimerMode) ?? 'timer');
  let hours = $state<number>((data.hours as number) ?? 0);
  let minutes = $state<number>((data.minutes as number) ?? 25);
  let seconds = $state<number>((data.seconds as number) ?? 0);
  let isCollapsed = $state((data.isCollapsed as boolean) ?? false);
  let notificationEnabled = $state((data.notificationEnabled as boolean) ?? false);
  let soundEnabled = $state((data.soundEnabled as boolean) ?? true);
  let showSeconds = $state((data.showSeconds as boolean) ?? true);

  let isRunning = $state((data.isRunning as boolean) ?? false);
  let isPaused = $state((data.isPaused as boolean) ?? false);
  let remainingSeconds = $state(0);
  let startTimestamp = $state<number | null>((data.startTimestamp as number) ?? null);
  let pausedTime = $state((data.pausedTime as number) ?? 0);
  let intervalId = $state<ReturnType<typeof setInterval> | null>(null);

  const totalInputSeconds = $derived(hours * 3600 + minutes * 60 + seconds);

  const currentSeconds = $derived(isRunning || isPaused ? remainingSeconds : mode === 'timer' ? totalInputSeconds : 0);

  const timeComponents = $derived({
    h: Math.floor(currentSeconds / 3600),
    m: Math.floor((currentSeconds % 3600) / 60),
    s: Math.floor(currentSeconds % 60),
  });

  const displayTime = $derived.by(() => {
    const { h, m, s } = timeComponents;
    const pad = (n: number) => n.toString().padStart(2, '0');

    if (h > 0) {
      return showSeconds ? `${pad(h)}:${pad(m)}:${pad(s)}` : `${pad(h)}:${pad(m)}`;
    }
    return showSeconds ? `${pad(m)}:${pad(s)}` : `${pad(m)}분`;
  });

  const toggleCollapse = () => {
    isCollapsed = !isCollapsed;
    saveSettings();
  };

  const saveSettings = () => {
    widgetContext.updateWidget?.(widgetId, {
      ...data,
      mode,
      hours,
      minutes,
      seconds,
      isCollapsed,
      notificationEnabled,
      soundEnabled,
      showSeconds,
      isRunning,
      isPaused,
      startTimestamp,
      pausedTime,
    });
  };

  const playBellSound = () => {
    const audioContext = new AudioContext();

    for (let i = 0; i < 3; i++) {
      const startTime = audioContext.currentTime + i * 0.3;
      const oscillator1 = audioContext.createOscillator();
      const oscillator2 = audioContext.createOscillator();
      const gainNode = audioContext.createGain();

      oscillator1.connect(gainNode);
      oscillator2.connect(gainNode);
      gainNode.connect(audioContext.destination);

      oscillator1.frequency.setValueAtTime(800, startTime);
      oscillator2.frequency.setValueAtTime(1200, startTime);
      oscillator1.type = 'sine';
      oscillator2.type = 'sine';

      gainNode.gain.setValueAtTime(0.3, startTime);
      gainNode.gain.exponentialRampToValueAtTime(0.01, startTime + 0.5);

      oscillator1.start(startTime);
      oscillator2.start(startTime);
      oscillator1.stop(startTime + 0.5);
      oscillator2.stop(startTime + 0.5);
    }

    setTimeout(() => {
      audioContext.close();
    }, 1200);
  };

  const notifyComplete = () => {
    if (notificationEnabled && 'Notification' in window && Notification.permission === 'granted') {
      new Notification('타이머 완료', {
        body: '설정한 시간이 종료되었어요.',
        icon: '/favicon.ico',
      });
    }

    if (soundEnabled) {
      playBellSound();
    }
  };

  const tick = () => {
    if (!isRunning || isPaused || startTimestamp === null) return;

    const elapsed = (Date.now() - startTimestamp) / 1000;

    if (mode === 'timer') {
      remainingSeconds = Math.max(0, totalInputSeconds - elapsed);

      if (remainingSeconds <= 0) {
        remainingSeconds = 0;
        isRunning = false;
        if (intervalId !== null) {
          clearInterval(intervalId);
          intervalId = null;
        }
        notifyComplete();
        saveSettings();
        return;
      }
    } else {
      remainingSeconds = elapsed;
    }
  };

  const start = () => {
    if (mode === 'timer' && totalInputSeconds === 0) return;

    if (!isRunning) {
      isRunning = true;
      isPaused = false;
      startTimestamp = Date.now();
      remainingSeconds = mode === 'timer' ? totalInputSeconds : 0;
      intervalId = setInterval(tick, 100);
    } else if (isPaused) {
      isPaused = false;
      const offset = mode === 'timer' ? totalInputSeconds - pausedTime : pausedTime;
      startTimestamp = Date.now() - offset * 1000;
      intervalId = setInterval(tick, 100);
    }

    saveSettings();
  };

  const pause = () => {
    if (isRunning && !isPaused) {
      isPaused = true;
      pausedTime = remainingSeconds;
      if (intervalId !== null) {
        clearInterval(intervalId);
        intervalId = null;
      }
      saveSettings();
    }
  };

  const reset = () => {
    isRunning = false;
    isPaused = false;
    remainingSeconds = mode === 'timer' ? totalInputSeconds : 0;
    startTimestamp = null;
    pausedTime = 0;

    if (intervalId !== null) {
      clearInterval(intervalId);
      intervalId = null;
    }

    saveSettings();
  };

  const handleModeChange = (value: TimerMode) => {
    if (isRunning) return;
    reset();
    mode = value;
    saveSettings();
  };

  const handleNotificationToggle = async () => {
    if (notificationEnabled) {
      notificationEnabled = false;
      saveSettings();
    } else {
      if ('Notification' in window) {
        const permission = await Notification.requestPermission();
        if (permission === 'granted') {
          notificationEnabled = true;
          saveSettings();
        }
      }
    }
  };

  const handleSoundToggle = () => {
    soundEnabled = !soundEnabled;
    saveSettings();
  };

  const handleShowSecondsToggle = () => {
    showSeconds = !showSeconds;
    saveSettings();
  };

  const clampInput = (value: string, max: number) => {
    const num = Number.parseInt(value, 10);
    return Number.isNaN(num) ? 0 : Math.max(0, Math.min(max, num));
  };

  $effect(() => {
    if (!isRunning || intervalId !== null) return;

    if (isPaused) {
      remainingSeconds = pausedTime;
    } else if (startTimestamp !== null) {
      const elapsed = (Date.now() - startTimestamp) / 1000;

      if (mode === 'timer') {
        remainingSeconds = Math.max(0, totalInputSeconds - elapsed);

        if (remainingSeconds <= 0) {
          remainingSeconds = 0;
          isRunning = false;
          notifyComplete();
          saveSettings();
          return;
        }
      } else {
        remainingSeconds = elapsed;
      }

      intervalId = setInterval(tick, 100);
    }
  });

  $effect(() => {
    return () => {
      if (intervalId !== null) {
        clearInterval(intervalId);
      }
    };
  });
</script>

<Widget collapsed={isCollapsed} icon={TimerIcon} title="타이머">
  {#snippet headerActions()}
    <button
      class={flex({ alignItems: 'center', gap: '2px', color: 'text.subtle', cursor: 'pointer' })}
      onclick={toggleCollapse}
      type="button"
    >
      {#if isCollapsed}
        <span class={css({ fontSize: '13px', fontWeight: 'normal', color: 'text.subtle' })}>
          {displayTime}
        </span>
      {/if}
      <Icon icon={isCollapsed ? ChevronDownIcon : ChevronUpIcon} size={14} />
    </button>
  {/snippet}

  <div
    class={flex({
      flexDirection: 'column',
      gap: '12px',
      alignItems: 'center',
    })}
  >
    <div class={flex({ justifyContent: 'space-between', alignItems: 'center', width: 'full' })}>
      <div class={css({ width: '110px' })}>
        <SegmentButtons
          style={css.raw({ opacity: isRunning ? '50' : '100', cursor: isRunning ? 'not-allowed' : 'auto' })}
          items={[
            { label: '타이머', value: 'timer' },
            { label: '스톱워치', value: 'stopwatch' },
          ]}
          onselect={handleModeChange}
          size="sm"
          value={mode}
        />
      </div>

      <div class={flex({ gap: '4px' })}>
        <button
          class={center({
            width: '28px',
            height: '28px',
            borderRadius: '6px',
            color: notificationEnabled ? 'text.subtle' : 'text.faint',
            cursor: 'pointer',
            transition: '[all 0.2s]',
            _hover: { backgroundColor: 'surface.subtle', color: 'text.default' },
          })}
          onclick={handleNotificationToggle}
          type="button"
          use:tooltip={{ message: notificationEnabled ? '알림 끄기' : '알림 켜기' }}
        >
          <Icon icon={notificationEnabled ? BellIcon : BellOffIcon} size={16} />
        </button>

        <button
          class={center({
            width: '28px',
            height: '28px',
            borderRadius: '6px',
            color: soundEnabled ? 'text.subtle' : 'text.faint',
            cursor: 'pointer',
            transition: '[all 0.2s]',
            _hover: { backgroundColor: 'surface.subtle', color: 'text.default' },
          })}
          onclick={handleSoundToggle}
          type="button"
          use:tooltip={{ message: soundEnabled ? '사운드 끄기' : '사운드 켜기' }}
        >
          <Icon icon={soundEnabled ? Volume2Icon : VolumeOffIcon} size={16} />
        </button>

        <button
          class={center({
            width: '28px',
            height: '28px',
            borderRadius: '6px',
            color: showSeconds ? 'text.subtle' : 'text.faint',
            cursor: 'pointer',
            transition: '[all 0.2s]',
            _hover: { backgroundColor: 'surface.subtle', color: 'text.default' },
          })}
          onclick={handleShowSecondsToggle}
          type="button"
          use:tooltip={{ message: showSeconds ? '초 숨기기' : '초 표시' }}
        >
          <Icon icon={ClockIcon} size={16} />
        </button>
      </div>
    </div>

    <div
      class={center({
        flexDirection: 'column',
        gap: '12px',
      })}
    >
      {#if mode === 'timer' && !isRunning && !isPaused}
        <div class={flex({ gap: '0', alignItems: 'center' })}>
          <input
            class={css({
              width: '64px',
              fontSize: '[40px]',
              fontWeight: 'extrabold',
              textAlign: 'right',
              backgroundColor: 'transparent',
              color: 'text.default',
              border: 'none',
              outline: 'none',
              fontVariantNumeric: 'tabular-nums',
              paddingX: '8px',
              borderRadius: '8px',
              _focus: { backgroundColor: 'surface.subtle' },
            })}
            max="23"
            min="0"
            onblur={(e) => {
              const target = e.target as HTMLInputElement;
              const value = target.value.replaceAll(/\D/g, '');
              if (value === '') {
                hours = 0;
              } else {
                hours = clampInput(value, 23);
              }
              target.value = hours.toString().padStart(2, '0');
              saveSettings();
            }}
            onfocus={(e) => {
              const target = e.target as HTMLInputElement;
              target.select();
            }}
            oninput={(e) => {
              const target = e.target as HTMLInputElement;
              let value = target.value.replaceAll(/\D/g, '');
              if (value.length > 2) {
                value = value.slice(-2);
              }
              hours = value === '' ? 0 : Number.parseInt(value, 10);
              target.value = hours.toString().padStart(2, '0');
            }}
            type="text"
            value={hours.toString().padStart(2, '0')}
          />
          <span class={css({ color: 'text.subtle', fontSize: '[40px]', fontWeight: 'extrabold' })}>:</span>
          <input
            class={css({
              width: '64px',
              fontSize: '[40px]',
              fontWeight: 'extrabold',
              textAlign: 'center',
              backgroundColor: 'transparent',
              color: 'text.default',
              border: 'none',
              outline: 'none',
              fontVariantNumeric: 'tabular-nums',
              paddingX: '8px',
              borderRadius: '8px',
              _focus: { backgroundColor: 'surface.subtle' },
            })}
            max="59"
            min="0"
            onblur={(e) => {
              const target = e.target as HTMLInputElement;
              const value = target.value.replaceAll(/\D/g, '');
              if (value === '') {
                minutes = 0;
              } else {
                minutes = clampInput(value, 59);
              }
              target.value = minutes.toString().padStart(2, '0');
              saveSettings();
            }}
            onfocus={(e) => {
              const target = e.target as HTMLInputElement;
              target.select();
            }}
            oninput={(e) => {
              const target = e.target as HTMLInputElement;
              let value = target.value.replaceAll(/\D/g, '');
              if (value.length > 2) {
                value = value.slice(-2);
              }
              minutes = value === '' ? 0 : Number.parseInt(value, 10);
              target.value = minutes.toString().padStart(2, '0');
            }}
            type="text"
            value={minutes.toString().padStart(2, '0')}
          />
          {#if showSeconds}
            <span class={css({ color: 'text.subtle', fontSize: '[40px]', fontWeight: 'extrabold' })}>:</span>
            <input
              class={css({
                width: '64px',
                fontSize: '[40px]',
                fontWeight: 'extrabold',
                textAlign: 'left',
                backgroundColor: 'transparent',
                color: 'text.default',
                border: 'none',
                outline: 'none',
                fontVariantNumeric: 'tabular-nums',
                paddingX: '8px',
                borderRadius: '8px',
                _focus: { backgroundColor: 'surface.subtle' },
              })}
              max="59"
              min="0"
              onblur={(e) => {
                const target = e.target as HTMLInputElement;
                const value = target.value.replaceAll(/\D/g, '');
                if (value === '') {
                  seconds = 0;
                } else {
                  seconds = clampInput(value, 59);
                }
                target.value = seconds.toString().padStart(2, '0');
                saveSettings();
              }}
              onfocus={(e) => {
                const target = e.target as HTMLInputElement;
                target.select();
              }}
              oninput={(e) => {
                const target = e.target as HTMLInputElement;
                let value = target.value.replaceAll(/\D/g, '');
                if (value.length > 2) {
                  value = value.slice(-2);
                }
                seconds = value === '' ? 0 : Number.parseInt(value, 10);
                target.value = seconds.toString().padStart(2, '0');
              }}
              type="text"
              value={seconds.toString().padStart(2, '0')}
            />
          {/if}
        </div>
      {:else}
        {@const { h, m, s } = timeComponents}
        <div class={flex({ gap: '0', alignItems: 'center' })}>
          <div
            class={css({
              width: '64px',
              fontSize: '[40px]',
              fontWeight: 'extrabold',
              textAlign: 'right',
              color: 'text.default',
              fontVariantNumeric: 'tabular-nums',
              paddingX: '8px',
            })}
          >
            {h.toString().padStart(2, '0')}
          </div>
          <span class={css({ color: 'text.subtle', fontSize: '[40px]', fontWeight: 'extrabold' })}>:</span>
          <div
            class={css({
              width: '64px',
              fontSize: '[40px]',
              fontWeight: 'extrabold',
              textAlign: 'center',
              color: 'text.default',
              fontVariantNumeric: 'tabular-nums',
              paddingX: '8px',
            })}
          >
            {m.toString().padStart(2, '0')}
          </div>
          {#if showSeconds}
            <span class={css({ color: 'text.subtle', fontSize: '[40px]', fontWeight: 'extrabold' })}>:</span>
            <div
              class={css({
                width: '64px',
                fontSize: '[40px]',
                fontWeight: 'extrabold',
                textAlign: 'left',
                color: 'text.default',
                fontVariantNumeric: 'tabular-nums',
                paddingX: '8px',
              })}
            >
              {s.toString().padStart(2, '0')}
            </div>
          {/if}
        </div>
      {/if}

      <div class={flex({ gap: '8px', alignItems: 'center', justifyContent: 'center', width: 'full' })}>
        {#if isRunning && !isPaused}
          <Button style={{ width: 'full', fontSize: '13px', whiteSpace: 'nowrap' }} onclick={pause} size="md" variant="secondary">
            <div class={flex({ gap: '4px', alignItems: 'center' })}>
              <Icon icon={PauseIcon} size={14} />
              일시정지
            </div>
          </Button>
        {:else if isPaused}
          <Button style={{ flex: '1', fontSize: '13px', whiteSpace: 'nowrap' }} onclick={reset} size="md" variant="secondary">
            <div class={flex({ gap: '4px', alignItems: 'center' })}>
              <Icon icon={RotateCcwIcon} size={14} />
              리셋
            </div>
          </Button>

          <Button style={{ flex: '1', fontSize: '13px', whiteSpace: 'nowrap' }} onclick={start} size="md" variant="secondary">
            <div class={flex({ gap: '4px', alignItems: 'center' })}>
              <Icon icon={PlayIcon} size={14} />
              재개
            </div>
          </Button>
        {:else}
          <Button style={{ width: 'full', fontSize: '13px' }} onclick={start} size="md" variant="primary">
            <div class={flex({ gap: '4px', alignItems: 'center' })}>
              <Icon icon={PlayIcon} size={14} />
              시작
            </div>
          </Button>
        {/if}
      </div>
    </div>
  </div>
</Widget>
