<script lang="ts">
  import { runningWorkflows } from '@typie/prism';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { toolCards } from './tools/index.ts';
  import type { ToolRequestMessage, Transcript } from '@typie/prism';

  type Props = {
    message: ToolRequestMessage;
    transcript: Transcript;
    failedIds: ReadonlySet<string>;
    resolve: (agentId: string, toolCallId: string, input: unknown) => Promise<void>;
    onRetry: (toolCallId: string) => void;
  };

  let { message, transcript, failedIds, resolve, onRetry }: Props = $props();

  const card = $derived(toolCards[message.tool]);

  const alive = $derived(
    message.workflowId === undefined
      ? transcript.run === 'running'
      : runningWorkflows(transcript).some((workflow) => workflow.workflowId === message.workflowId),
  );
  const open = $derived(message.status === 'pending' && alive);
  const failed = $derived(message.status === 'pending' && failedIds.has(message.toolCallId));
</script>

{#if card}
  {@const Card = card}
  <Card {message} {open} resolve={(input) => resolve(message.agentId, message.toolCallId, input)} />
{:else if failed}
  <div class={flex({ alignItems: 'center', gap: '8px' })}>
    <span class={css({ fontSize: '11px', color: 'text.danger' })}>요청을 처리하지 못했어요</span>
    <Button onclick={() => onRetry(message.toolCallId)} size="sm" variant="secondary">다시 시도</Button>
  </div>
{/if}
