<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { tick, untrack } from 'svelte';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import PyramidIcon from '~icons/lucide/pyramid';
  import { PacedText } from './lib/paced-text.svelte.ts';
  import PrismMarkdown from './PrismMarkdown.svelte';
  import PrismMessage from './PrismMessage.svelte';
  import type { Transcript } from './lib/conversation.ts';

  type Props = {
    transcript: Transcript;
    loading: boolean;
    pending: string | null;
  };

  let { transcript, loading, pending }: Props = $props();

  const MAX_FRAME_MS = 100;

  const reduceMotion = typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const newPaced = () => new PacedText({ instant: reduceMotion });

  let container = $state<HTMLElement>();
  let follow = $state(true);
  let live = $state<PacedText | null>(null);
  let drains = $state<{ key: string; paced: PacedText }[]>([]);
  let liveKey = '';

  $effect.pre(() => {
    const turn = transcript.live;
    const key = turn ? `${turn.context.agent.id}:${turn.context.run}:${turn.context.turn}:${turn.context.attempt}` : '';

    if (key !== liveKey) {
      untrack(() => {
        if (live !== null && key === '') {
          const sealed = transcript.messages.at(-1);
          if (sealed?.role === 'assistant' && sealed.text !== null && live.boundary > 0) {
            live.finalize(sealed.text);
            if (!live.done) drains = [...drains, { key: sealed.key, paced: live }];
          }
        }

        liveKey = key;
        live = key === '' ? null : newPaced();
      });
    }

    if (turn && live !== null) live.retarget(turn.textBroken ? '' : turn.text);
  });

  $effect.pre(() => {
    const alive = drains.filter(({ key }) => transcript.messages.some((message) => message.key === key));
    if (alive.length !== untrack(() => drains.length)) drains = alive;
  });

  const active = $derived(live !== null || drains.length > 0);

  $effect(() => {
    if (!active) {
      return;
    }

    let disposed = false;
    let last = performance.now();
    let id = requestAnimationFrame(function loop(time) {
      if (disposed) {
        return;
      }

      const dt = Math.min(time - last, MAX_FRAME_MS);
      last = time;
      live?.advance(dt);
      let settled = false;
      for (const { paced } of drains) {
        paced.advance(dt);
        settled ||= paced.done;
      }
      if (settled) drains = drains.filter(({ paced }) => !paced.done);

      // 추종 스크롤 스무딩 — 콘텐츠는 줄 단위로 자라므로 즉시 붙이면 줄바꿈마다 튄다.
      if (follow && container) {
        const target = container.scrollHeight - container.clientHeight;
        const gap = target - container.scrollTop;
        if (gap > 0) container.scrollTop = gap < 1 ? target : container.scrollTop + gap * (1 - Math.exp(-dt / 120));
      }

      id = requestAnimationFrame(loop);
    });

    return () => {
      disposed = true;
      cancelAnimationFrame(id);
    };
  });

  const drainOf = (key: string) => drains.find((drain) => drain.key === key);

  let prevPending: string | null = null;

  $effect.pre(() => {
    if (pending !== null && prevPending === null) follow = true;
    prevPending = pending;
  });

  $effect(() => {
    void transcript.messages.length;
    void live?.boundary;
    for (const drain of drains) void drain.paced.boundary;
    void pending;

    const element = container;
    if (!element || active || !untrack(() => follow)) {
      return;
    }

    void tick().then(() => element.scrollTo({ top: element.scrollHeight }));
  });

  const shimmer = css({
    width: '[fit-content]',
    color: '[transparent]',
    backgroundImage: '[linear-gradient(90deg, token(colors.text.faint) 30%, token(colors.text.default) 50%, token(colors.text.faint) 70%)]',
    backgroundSize: '[200% 100%]',
    backgroundClip: 'text',
    animation: '[shimmer 1.8s linear infinite]',
    _motionReduce: { animation: 'none', color: 'text.faint', backgroundImage: 'none' },
  });

  // 해제는 방향으로 판정한다 — 추종 스크롤은 항상 아래로만 움직이므로, scrollTop 감소는 사용자가
  // 위로 올렸다는 뜻이다(콘텐츠 축소로 인한 클램프 감소는 바닥 밀착이라 gap 조건이 거른다).
  // 재추종은 사용자가 실제 바닥에 닿았을 때만 — 근처 문턱으로 미리 낚아채면 스냅으로 느껴진다.
  let lastTop = 0;

  const onScroll = () => {
    const element = container;
    if (!element) {
      return;
    }

    const gap = element.scrollHeight - element.scrollTop - element.clientHeight;
    if (element.scrollTop < lastTop - 1 && gap >= 8) {
      follow = false;
    } else if (gap < 8) {
      follow = true;
    }

    lastTop = element.scrollTop;
    updateOverflow();
  };

  let content = $state<HTMLElement>();
  let overflowTop = $state(false);
  let overflowBottom = $state(false);

  const updateOverflow = () => {
    if (!container) return;
    overflowTop = container.scrollTop > 1;
    overflowBottom = container.scrollTop + container.clientHeight < container.scrollHeight - 1;
  };

  $effect(() => {
    if (!container || !content) return;
    const observer = new ResizeObserver(updateOverflow);
    observer.observe(container);
    observer.observe(content);
    return () => observer.disconnect();
  });

  const maskImage = $derived.by(() => {
    if (!overflowTop && !overflowBottom) return;
    const from = overflowTop ? 'transparent, black 24px' : 'black, black 24px';
    const to = overflowBottom ? 'black calc(100% - 24px), transparent' : 'black calc(100% - 24px), black';
    return `linear-gradient(to bottom, ${from}, ${to})`;
  });
</script>

<div class={flex({ position: 'relative', flexDirection: 'column', flexGrow: '1', minHeight: '0' })}>
  <div
    bind:this={container}
    style:mask-image={maskImage}
    class={flex({
      flexDirection: 'column',
      flexGrow: '1',
      minHeight: '0',
      overflowY: 'auto',
      scrollbarWidth: 'none',
      paddingX: '16px',
      paddingY: '16px',
    })}
    onscroll={onScroll}
  >
    <div bind:this={content} class={flex({ flexDirection: 'column', gap: '18px', flexGrow: '1' })}>
      {#if loading && pending === null}
        <div
          class={css({
            height: '14px',
            width: '[60%]',
            borderRadius: '4px',
            backgroundColor: 'surface.muted',
            animation: 'pulse 1.6s ease-in-out infinite',
          })}
        ></div>
      {:else if transcript.messages.length === 0 && !transcript.live && pending === null}
        <div class={flex({ flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: '10px', flexGrow: '1' })}>
          <Icon style={css.raw({ color: 'border.default' })} icon={PyramidIcon} size={32} />
        </div>
      {/if}

      {#each transcript.messages as message (message.key)}
        {@const drain = drainOf(message.key)}
        {#if drain}
          <PrismMarkdown blocks={drain.paced.blocks} plain={drain.paced.plain} />
        {:else}
          <PrismMessage {message} />
        {/if}
      {/each}

      {#if pending !== null}
        <div
          class={css({
            alignSelf: 'flex-end',
            maxWidth: '[86%]',
            paddingX: '12px',
            paddingY: '8px',
            borderRadius: '12px',
            borderBottomRightRadius: '2px',
            backgroundColor: 'surface.muted',
            fontSize: '14px',
            lineHeight: '[1.6]',
            whiteSpace: 'pre-wrap',
            animation: '[rise-in 200ms cubic-bezier(0.23, 1, 0.32, 1) both]',
            _motionReduce: { animation: 'none' },
          })}
        >
          {pending}
        </div>
      {/if}

      {#if pending !== null && !transcript.live && transcript.run !== 'running'}
        <div class={css({ fontSize: '14px' })}>
          <span class={shimmer}>생각하는 중…</span>
        </div>
      {/if}

      {#if transcript.live && live}
        {#if live.boundary === 0 && transcript.live.toolInput}
          <div class={css({ fontSize: '14px' })}>
            <span class={shimmer}>{transcript.live.toolInput.name} 준비 중…</span>
          </div>
        {:else if live.boundary === 0 && transcript.live.thinkingChars > 0}
          <div class={css({ fontSize: '14px' })}>
            <span class={shimmer}>생각하는 중…</span>
          </div>
        {:else if live.boundary > 0}
          <PrismMarkdown blocks={live.blocks} plain={live.plain} />
        {/if}
      {:else if transcript.run === 'running'}
        <div class={css({ fontSize: '14px' })}>
          <span class={shimmer}>{transcript.retrying ? '다시 시도하는 중…' : '생각하는 중…'}</span>
        </div>
      {/if}

      {#if transcript.run === 'failed'}
        <div class={css({ alignSelf: 'center', fontSize: '11px', color: 'text.danger' })}>응답을 마치지 못했어요 — 다시 보내 주세요</div>
      {:else if transcript.run === 'canceled'}
        <div class={css({ alignSelf: 'center', fontSize: '11px', color: 'text.faint' })}>중단됨</div>
      {/if}
    </div>
  </div>

  {#if !follow}
    <button
      class={css({
        position: 'absolute',
        bottom: '12px',
        left: '[calc(50% - 14px)]',
        size: '28px',
        borderRadius: 'full',
        backgroundColor: 'surface.dark',
        color: 'text.bright',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        animation: '[rise-in 200ms cubic-bezier(0.23, 1, 0.32, 1) both]',
        transition: '[transform 160ms cubic-bezier(0.23, 1, 0.32, 1)]',
        _active: { transform: 'scale(0.97)' },
        _motionReduce: { animation: 'none' },
      })}
      aria-label="아래로"
      onclick={() => {
        follow = true;
        container?.scrollTo({ top: container.scrollHeight, behavior: reduceMotion ? 'auto' : 'smooth' });
      }}
      type="button"
    >
      <Icon icon={ChevronDownIcon} size={16} />
    </button>
  {/if}
</div>
