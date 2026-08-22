<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { tick, untrack } from 'svelte';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import PyramidIcon from '~icons/lucide/pyramid';
  import { pop, rise } from './lib/motion.ts';
  import { PacedText } from './lib/paced-text.svelte.ts';
  import { foldToolCalls } from './lib/tool-calls.ts';
  import PrismMarkdown from './PrismMarkdown.svelte';
  import PrismMessage from './PrismMessage.svelte';
  import PrismToolCalls from './PrismToolCalls.svelte';
  import PrismToolRequest from './PrismToolRequest.svelte';
  import PrismWaitRow from './PrismWaitRow.svelte';
  import PrismWorkflow from './PrismWorkflow.svelte';
  import { toolCallLabels, toolCards } from './tools/index.ts';
  import type { Transcript, TranscriptMessage } from './lib/conversation.ts';

  type Props = {
    transcript: Transcript;
    loading: boolean;
    pending: string | null;
    sessionId: string | null;
    failedIds: ReadonlySet<string>;
    reconnecting: boolean;
    onResolve: (agentId: string, toolCallId: string, input: unknown) => Promise<void>;
    onRetry: (toolCallId: string) => void;
    onLoadTrace: (workflowId: string) => Promise<void>;
  };

  let { transcript, loading, pending, sessionId, failedIds, reconnecting, onResolve, onRetry, onLoadTrace }: Props = $props();

  const foldable = (message: TranscriptMessage) =>
    (message.role === 'tool' && message.phase === 'executed' && message.ok !== false) ||
    (message.role === 'tool-request' &&
      message.workflowId === undefined &&
      toolCards[message.tool] === undefined &&
      message.status !== 'pending');

  const labelOf = (message: TranscriptMessage) =>
    message.role === 'tool' || message.role === 'tool-request'
      ? (toolCallLabels[message.role === 'tool' ? message.name : message.tool] ?? null)
      : null;

  const dropped = (message: TranscriptMessage) =>
    (message.role === 'tool-request' && message.workflowId !== undefined) || (message.role === 'tool' && !foldable(message));

  const entries = $derived(
    foldToolCalls(
      transcript.messages.filter((message) => !dropped(message)),
      foldable,
      labelOf,
    ),
  );

  let settled = $state(false);

  $effect(() => {
    if (loading) {
      return;
    }

    void tick().then(() => (settled = true));
  });

  const entryClass = flex({ flexDirection: 'column' });

  const LATE_MS = 30_000;

  let lastBeat = $state(Date.now());
  let nowTick = $state(Date.now());

  $effect.pre(() => {
    void transcript.cursor;
    void transcript.live;
    void pending;
    lastBeat = Date.now();
  });

  const ticking = $derived(pending !== null || transcript.turn === 'active');

  $effect(() => {
    if (!ticking) {
      return;
    }

    const id = setInterval(() => (nowTick = Date.now()), 1000);
    return () => clearInterval(id);
  });

  const late = $derived(nowTick - lastBeat >= LATE_MS);

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

        if (live !== null && turn?.seeded) {
          live.retarget(turn.textBroken ? '' : turn.text);
          live.skip();
        }
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
    const scroller = container;
    if (!scroller || !content) return;
    const observer = new ResizeObserver(() => {
      if (follow && !active) {
        scroller.scrollTop = scroller.scrollHeight - scroller.clientHeight;
      }
      updateOverflow();
    });
    observer.observe(scroller);
    observer.observe(content);
    return () => observer.disconnect();
  });

  type WaitState = { label: string; text: string | null } | null;

  const waitState = $derived.by<WaitState>(() => {
    const turn = transcript.live;

    if (turn && live) {
      if (live.boundary > 0) return null;
      if (late) return { label: '응답이 늦어지고 있어요', text: '응답이 늦어지고 있어요' };
      if (turn.last === 'thinking' && turn.thinkingChars > 0) return { label: '생각하는 중', text: '생각하는 중' };
      if (turn.last === 'tool.input' && turn.toolInput) return { label: '도구를 준비하는 중', text: null };
      return { label: '응답을 기다리는 중', text: null };
    }

    if (late && (pending !== null || transcript.turn === 'active')) {
      return { label: '응답이 늦어지고 있어요', text: '응답이 늦어지고 있어요' };
    }

    if (pending !== null && transcript.run !== 'running') return { label: '보내는 중', text: null };
    if (transcript.turn === 'active') return { label: transcript.retrying ? '다시 시도하는 중' : '응답을 기다리는 중', text: null };

    return null;
  });

  const WAIT_DEBOUNCE_MS = 1000;
  const rawWaitText = $derived(waitState?.text ?? null);
  let waitText = $state<string | null>(null);

  $effect(() => {
    const next = rawWaitText;
    waitText = null;

    if (next === null) {
      return;
    }

    const id = setTimeout(() => (waitText = next), WAIT_DEBOUNCE_MS);
    return () => clearTimeout(id);
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

      {#each entries as entry (entry.key)}
        {@const drain = drainOf(entry.key)}
        <div class={entryClass} in:rise={{ skip: !settled || entry.role === 'user' || entry.role === 'assistant' }}>
          {#if entry.role === 'tool-calls'}
            <PrismToolCalls count={entry.count} rows={entry.rows} />
          {:else if drain}
            <PrismMarkdown blocks={drain.paced.blocks} plain={drain.paced.plain} />
          {:else if entry.role === 'tool-request'}
            <PrismToolRequest {failedIds} message={entry} {onRetry} resolve={onResolve} {transcript} />
          {:else if entry.role === 'run-failed'}
            <div class={css({ alignSelf: 'center', fontSize: '11px', color: 'text.danger' })}>
              응답을 마치지 못했어요 — 다시 보내 주세요
            </div>
          {:else if entry.role === 'workflow'}
            {#if sessionId !== null}
              <PrismWorkflow
                {failedIds}
                loadTrace={onLoadTrace}
                message={entry}
                {onRetry}
                {reconnecting}
                resolve={onResolve}
                {sessionId}
                {transcript}
              />
            {/if}
          {:else}
            <PrismMessage message={entry} />
          {/if}
        </div>
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

      {#if transcript.live && live && live.boundary > 0}
        <PrismMarkdown blocks={live.blocks} plain={live.plain} />
      {/if}

      {#if waitState !== null}
        <PrismWaitRow label={waitState.label} text={waitText} />
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
        transition: '[transform 160ms cubic-bezier(0.23, 1, 0.32, 1)]',
        _active: { transform: 'scale(0.97)' },
      })}
      aria-label="아래로"
      onclick={() => {
        follow = true;
        container?.scrollTo({ top: container.scrollHeight, behavior: reduceMotion ? 'auto' : 'smooth' });
      }}
      type="button"
      in:pop
      out:pop={{ out: true }}
    >
      <Icon icon={ChevronDownIcon} size={16} />
    </button>
  {/if}
</div>
