<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { workflowApps } from './workflows/index.ts';
  import type { ToolRequestMessage, Transcript, WorkflowMessage } from './lib/conversation.ts';

  type Props = {
    message: WorkflowMessage;
    sessionId: string;
    transcript: Transcript;
    failedIds: ReadonlySet<string>;
    reconnecting: boolean;
    resolve: (agentId: string, toolCallId: string, input: unknown) => Promise<void>;
    onRetry: (toolCallId: string) => void;
    loadTrace: (workflowId: string) => Promise<void>;
  };

  let { message, sessionId, transcript, failedIds, reconnecting, resolve, onRetry, loadTrace }: Props = $props();

  const app = $derived(workflowApps[message.app]);
  const requests = $derived(
    transcript.messages.filter((m): m is ToolRequestMessage => m.role === 'tool-request' && m.workflowId === message.workflowId),
  );

  let now = $state(Date.now());

  $effect(() => {
    if (app !== undefined || message.status !== 'running') {
      return;
    }

    const id = setInterval(() => (now = Date.now()), 60_000);
    return () => clearInterval(id);
  });

  const minutes = $derived(Math.max(0, Math.floor((now - message.startedAt) / 60_000)));
</script>

{#if app}
  {@const Block = app.block}
  <Block
    {failedIds}
    loadTrace={() => loadTrace(message.workflowId)}
    {message}
    {onRetry}
    {reconnecting}
    {requests}
    {resolve}
    {sessionId}
    {transcript}
  />
{:else if message.status === 'running'}
  <div class={flex({ alignItems: 'center', gap: '6px', fontSize: '12px', color: 'text.subtle' })}>
    <div
      class={css({
        size: '5px',
        borderRadius: 'full',
        flexShrink: '0',
        backgroundColor: 'border.strong',
        animation: 'pulse 1.6s ease-in-out infinite',
      })}
    ></div>
    작업 진행 중 · 경과 {minutes}분
  </div>
{:else}
  <div class={css({ fontSize: '12px', color: 'text.faint' })}>작업이 끝났어요</div>
{/if}
