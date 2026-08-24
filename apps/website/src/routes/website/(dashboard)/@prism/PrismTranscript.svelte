<script lang="ts">
  import { pendingRootRequests, runningWorkflows } from '@typie/prism';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { tick, untrack } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import { parseMarkdown } from './lib/markdown.ts';
  import { pop, rise } from './lib/motion.ts';
  import { PacedText } from './lib/paced-text.svelte.ts';
  import { foldToolCalls } from './lib/tool-calls.ts';
  import PrismMarkdown from './PrismMarkdown.svelte';
  import PrismMessage from './PrismMessage.svelte';
  import PrismToolCalls from './PrismToolCalls.svelte';
  import PrismToolRequest from './PrismToolRequest.svelte';
  import PrismWaitRow from './PrismWaitRow.svelte';
  import PrismWorkflow from './PrismWorkflow.svelte';
  import { clientResolvers, toolCallLabels, toolCards } from './tools/index.ts';
  import type { Transcript, TranscriptMessage } from '@typie/prism';
  import type { BlockNode } from './lib/markdown.ts';

  type Props = {
    transcript: Transcript;
    loading: boolean;
    pending: string | null;
    sessionId: string | null;
    failedIds: ReadonlySet<string>;
    reconnecting: boolean;
    spinnerOwner?: 'panel' | 'row';
    waitSpinnerAnchor?: HTMLElement;
    onResolve: (agentId: string, toolCallId: string, input: unknown) => Promise<void>;
    onRetry: (toolCallId: string) => void;
  };

  let {
    transcript,
    loading,
    pending,
    sessionId,
    failedIds,
    reconnecting,
    spinnerOwner = 'row',
    waitSpinnerAnchor = $bindable(),
    onResolve,
    onRetry,
  }: Props = $props();

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
    (message.role === 'tool-request' && message.workflowId !== undefined) ||
    (message.role === 'tool' && !foldable(message)) ||
    (message.role === 'assistant' && message.text === null) ||
    (message.role === 'tool-request' &&
      !foldable(message) &&
      toolCards[message.tool] === undefined &&
      !(message.status === 'pending' && failedIds.has(message.toolCallId)));

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
  let chase = $state(false);

  let anchored = false;

  const chaseBottom = (element: HTMLElement) => {
    if (reduceMotion || !anchored) {
      anchored = !loading;
      element.scrollTop = element.scrollHeight - element.clientHeight;
      return;
    }

    chase = true;
  };

  let live = $state<PacedText | null>(null);
  let drains = $state<{ key: string; paced: PacedText }[]>([]);
  let liveKey = '';
  const aliases = new SvelteMap<string, string>();

  $effect.pre(() => {
    const turn = transcript.live;
    const key = turn ? `${turn.context.agent.id}:${turn.context.run}:${turn.context.turn}:${turn.context.attempt}` : '';

    if (key !== liveKey) {
      untrack(() => {
        // 봉인 키(e{seq})는 스트리밍 중 예측할 수 없다 — 전환 시점에 별칭을 남겨 each 항목이 라이브→봉인을 한 항목으로 잇는다.
        if (liveKey !== '' && (live?.boundary ?? 0) > 0) {
          for (let i = transcript.messages.length - 1; i >= 0; i--) {
            const message = transcript.messages[i];
            if (message.role === 'assistant') {
              if (message.text !== null && !aliases.has(message.key)) aliases.set(message.key, liveKey);
              break;
            }
          }
        }

        if (key === '') {
          const sealed = transcript.messages.at(-1);
          // 프레임이 뭉쳐 도착하면 live가 만들어지기 전에 턴이 닫힌다 — 실시간 여부는 모델이 기록한 streamed로 판정한다.
          if (sealed?.role === 'assistant' && sealed.text !== null && (live?.boundary ?? 0) > 0) {
            live?.finalize(sealed.text);
            if (live !== null && !live.done) drains = [...drains, { key: sealed.key, paced: live }];
          } else if (sealed?.role === 'assistant' && sealed.text !== null && sealed.streamed) {
            const paced = live ?? newPaced();
            paced.finalize(sealed.text);
            if (!paced.done) drains = [...drains, { key: sealed.key, paced }];
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
  const draining = $derived(drains.some((entry) => !entry.paced.done));

  // 배출이 끝나기 전에 뒤 블록을 세우면 표시 순서가 역전되고, 자라는 텍스트가 그 블록을 밀어낸다.
  const gated = $derived.by(() => {
    const pending = entries.findIndex((entry) => {
      const drain = drains.find((entry_) => entry_.key === entry.key);
      return drain !== undefined && !drain.paced.done;
    });

    return pending === -1 ? entries : entries.slice(0, pending + 1);
  });

  $effect(() => {
    if (!active && !chase) {
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
        if (!active && gap < 1) chase = false;
      } else if (!active) {
        chase = false;
      }

      id = requestAnimationFrame(loop);
    });

    return () => {
      disposed = true;
      cancelAnimationFrame(id);
    };
  });

  const drainOf = (key: string) => drains.find((drain) => drain.key === key);

  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- 봉인 텍스트는 키별 불변이라 비반응 메모로 충분하다
  const parsed = new Map<string, BlockNode[]>();
  const parsedBlocks = (key: string, text: string) => {
    let blocks = parsed.get(key);
    if (blocks === undefined) {
      blocks = parseMarkdown(text);
      parsed.set(key, blocks);
    }
    return blocks;
  };

  type RenderEntry = (typeof entries)[number];
  type RenderItem =
    | { kind: 'markdown'; key: string; blocks: BlockNode[]; plain: number; settled: boolean }
    | { kind: 'entry'; key: string; entry: RenderEntry };

  // assistant 마크다운은 라이브·드레인·봉인 세 국면을 같은 키의 같은 항목으로 잇는다 — 위치 이동 재마운트가 카드를 한 프레임 스켈레톤으로 되돌리지 않도록.
  const rendered = $derived.by<RenderItem[]>(() => {
    const items: RenderItem[] = gated.map((entry) => {
      if (entry.role === 'assistant' && entry.text !== null) {
        const drain = drainOf(entry.key);
        const key = aliases.get(entry.key) ?? entry.key;
        return drain
          ? { kind: 'markdown', key, blocks: drain.paced.blocks, plain: drain.paced.plain, settled: false }
          : { kind: 'markdown', key, blocks: parsedBlocks(entry.key, entry.text), plain: Number.MAX_SAFE_INTEGER, settled: true };
      }

      return { kind: 'entry', key: entry.key, entry };
    });

    if (live && !draining && transcript.live && live.boundary > 0) {
      items.push({ kind: 'markdown', key: liveKey, blocks: live.blocks, plain: live.plain, settled: false });
    }

    return items;
  });

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

    void tick().then(() => chaseBottom(element));
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
        chaseBottom(scroller);
      }
      updateOverflow();
    });
    observer.observe(scroller);
    observer.observe(content);
    return () => observer.disconnect();
  });

  type WaitState = { label: string; text: string | null } | null;

  const RECONNECTING_WAIT = { label: '다시 연결하는 중', text: '다시 연결하는 중' };

  const workflowActive = $derived(runningWorkflows(transcript).length > 0);
  const awaitingUser = $derived(pendingRootRequests(transcript).some((request) => clientResolvers[request.tool] === undefined));

  const waitState = $derived.by<WaitState>(() => {
    if (awaitingUser) {
      return null;
    }

    // 드레인이 텍스트를 그리는 동안은 대기가 아니라 전달이다 — turn.completed와 run.completed 사이 창에서 스피너가 깜빡이지 않도록 (gated와 같은 원리).
    if (draining) {
      return null;
    }

    const turn = transcript.live;

    if (turn && live) {
      if (live.boundary > 0) return null;
      if (reconnecting) return RECONNECTING_WAIT;
      if (late) return { label: '응답이 늦어지고 있어요', text: '응답이 늦어지고 있어요' };
      if (turn.last === 'thinking' && turn.thinkingChars > 0) return { label: '생각하는 중', text: '생각하는 중' };
      if (turn.last === 'tool.input' && turn.toolInput) return { label: '도구를 준비하는 중', text: null };
      return { label: '응답을 기다리는 중', text: null };
    }

    if (workflowActive) {
      return null;
    }

    if (pending === null && transcript.turn !== 'active' && transcript.run !== 'running') {
      return null;
    }

    if (reconnecting) return RECONNECTING_WAIT;
    if (late) return { label: '응답이 늦어지고 있어요', text: '응답이 늦어지고 있어요' };

    if (pending !== null && transcript.run !== 'running') return { label: '보내는 중', text: null };
    if (transcript.retrying) return { label: '다시 시도하는 중', text: null };

    return { label: '응답을 기다리는 중', text: null };
  });

  const WAIT_DEBOUNCE_MS = 1000;
  const rawWaitText = $derived(waitState?.text ?? null);
  let waitText = $state<string | null>(null);

  $effect(() => {
    const next = rawWaitText;

    if (next === null) {
      waitText = null;
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
      {/if}

      {#each rendered as item (item.key)}
        <div
          class={entryClass}
          in:rise={{
            skip: !settled || item.kind === 'markdown' || item.entry.role === 'user',
            block: item.kind === 'markdown' || item.entry.role !== 'tool-calls',
          }}
        >
          {#if item.kind === 'markdown'}
            <PrismMarkdown blocks={item.blocks} plain={item.plain} settled={item.settled} />
          {:else if item.entry.role === 'tool-calls'}
            <PrismToolCalls count={item.entry.count} rows={item.entry.rows} />
          {:else if item.entry.role === 'tool-request'}
            <PrismToolRequest {failedIds} message={item.entry} {onRetry} resolve={onResolve} {transcript} />
          {:else if item.entry.role === 'run-failed'}
            <div class={css({ alignSelf: 'center', fontSize: '11px', color: 'text.danger' })}>응답을 마치지 못했어요. 다시 보내 주세요</div>
          {:else if item.entry.role === 'workflow'}
            {#if sessionId !== null}
              <PrismWorkflow {failedIds} message={item.entry} {onRetry} {reconnecting} resolve={onResolve} {sessionId} {transcript} />
            {/if}
          {:else}
            <PrismMessage message={item.entry} />
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

      {#if waitState !== null}
        <PrismWaitRow label={waitState.label} {spinnerOwner} text={waitText} bind:spinnerAnchor={waitSpinnerAnchor} />
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
