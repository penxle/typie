<script lang="ts">
  import { runningWorkflows } from '@typie/prism';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button } from '@typie/ui/components';
  import { toolCards } from './tools/index.ts';
  import type { ToolRequestMessage, Transcript } from '@typie/prism';

  type Props = {
    message: ToolRequestMessage;
    sessionId: string | null;
    transcript: Transcript;
    failedIds: ReadonlySet<string>;
    unavailableMessage?: string;
    resolve: (agentId: string, toolCallId: string, input: unknown) => Promise<void>;
    onRetry: (toolCallId: string) => void;
  };

  let { message, sessionId, transcript, failedIds, unavailableMessage, resolve, onRetry }: Props = $props();

  const card = $derived(toolCards[message.tool]);

  const alive = $derived(
    message.workflowId === undefined
      ? transcript.run === 'running'
      : runningWorkflows(transcript).some((workflow) => workflow.workflowId === message.workflowId),
  );
  const open = $derived(message.status === 'pending' && alive);
  const guarded = $derived(unavailableMessage !== undefined && open);
  const failed = $derived(message.status === 'pending' && failedIds.has(message.toolCallId));
</script>

{#if card}
  {@const Card = card}
  <div use:tooltip={{ message: guarded ? unavailableMessage : null, placement: 'bottom', delay: 0, arrow: false }}>
    <div class={css({ '&[data-readonly=true]': { opacity: '40' } })} aria-disabled={guarded} data-readonly={guarded} inert={guarded}>
      <Card disabled={guarded} {message} {open} resolve={(input) => resolve(message.agentId, message.toolCallId, input)} {sessionId} />
    </div>
  </div>
{:else if failed}
  <div
    class={flex({ alignItems: 'center', gap: '8px' })}
    use:tooltip={{ message: guarded ? unavailableMessage : null, placement: 'bottom', delay: 0, arrow: false }}
  >
    <span class={css({ fontSize: '11px', color: 'danger.default' })}>요청을 처리하지 못했어요</span>
    <Button disabled={guarded} onclick={() => onRetry(message.toolCallId)} size="sm" variant="secondary">다시 시도</Button>
  </div>
{/if}
