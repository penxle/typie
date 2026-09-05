<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { quintOut } from 'svelte/easing';
  import type { TransitionConfig } from 'svelte/transition';

  const WELCOME_MESSAGES = [
    '도울 일이 있다면 맡겨주세요.',
    '어디부터 함께 볼까요?',
    '무엇부터 살펴볼까요?',
    '무엇을 함께 생각해볼까요?',
    '어떤 글을 쓰고 있나요?',
    '무엇을 함께 해볼까요?',
    '어디부터 손을 보탤까요?',
    '손이 필요한 일이라면 맡겨주세요.',
    '어떤 원고부터 펼쳐볼까요?',
    '오늘은 무엇부터 시작할까요?',
    '어디에 빛을 비춰볼까요?',
    '같은 글도 다르게 읽힐 수 있어요.',
    '어떤 이야기를 새롭게 바라볼까요?',
    '무엇을 다른 각도에서 볼까요?',
    '아직 보이지 않는 걸 찾아볼까요?',
  ] as const;
  const ENTER_DURATION_MS = 420;
  const EXIT_DURATION_MS = 150;

  type Props = {
    delayMs: number;
    immediate?: boolean;
    visible: boolean;
  };

  let { delayMs, immediate = false, visible }: Props = $props();
  const message = WELCOME_MESSAGES[Math.floor(Math.random() * WELCOME_MESSAGES.length)];

  const reveal = (node: Element, { delay, skip }: { delay: number; skip: boolean }): TransitionConfig => {
    if (skip) return { duration: 0 };
    const element = node as HTMLElement;
    return {
      delay,
      duration: ENTER_DURATION_MS,
      easing: quintOut,
      tick: (progress, inverseProgress) => {
        const done = progress === 1;
        element.style.opacity = done ? '' : String(progress);
        element.style.transform = done ? '' : `translateY(${inverseProgress * 6}px)`;
        element.style.filter = done ? '' : `blur(${inverseProgress * 2}px)`;
      },
    };
  };

  const hide = (node: Element, { skip }: { skip: boolean }): TransitionConfig => {
    if (skip) return { duration: 0 };
    const element = node as HTMLElement;
    return {
      duration: EXIT_DURATION_MS,
      easing: quintOut,
      tick: (progress) => {
        element.style.opacity = String(progress);
      },
    };
  };
</script>

{#if visible}
  <p
    class={css({
      position: 'absolute',
      top: '[calc(50% + 60px)]',
      left: '0',
      zIndex: '1',
      width: 'full',
      paddingX: '40px',
      overflowWrap: 'break-word',
      fontSize: '15px',
      fontWeight: 'medium',
      lineHeight: '[26px]',
      textAlign: 'center',
      textWrap: 'balance',
      wordBreak: 'keep-all',
      color: 'text.muted',
      pointerEvents: 'none',
      willChange: 'opacity, transform, filter',
    })}
    data-prism-indicator-message
    in:reveal={{ delay: delayMs, skip: immediate }}
    out:hide={{ skip: immediate }}
  >
    {message}
  </p>
{/if}
