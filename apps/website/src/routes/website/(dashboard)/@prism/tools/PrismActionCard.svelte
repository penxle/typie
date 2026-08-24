<script lang="ts">
  import { TypieError } from '@typie/lib/errors';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import { unwrapError } from '$lib/graphql/error';
  import { actionCards, actionOutcome, actionTail } from './action-cards.ts';
  import type { ToolCardProps } from './index.ts';

  let { message, open, resolve }: ToolCardProps = $props();

  const entry = $derived(actionCards[message.tool]);

  let busy = $state(false);
  let ready = $state(false);
  let sendFailed = $state(false);

  const setReady = (value: boolean) => (ready = value);

  const outcome = $derived(actionOutcome(message));

  const submit = async (approve: boolean) => {
    if (busy) {
      return;
    }

    busy = true;
    sendFailed = false;

    try {
      await resolve({ approve });
    } catch (err) {
      const error = unwrapError(err);
      if (!(error instanceof TypieError && error.code === 'prism_tool_settled')) sendFailed = true;
    } finally {
      busy = false;
    }
  };

  const cardClass = css({
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '10px',
    padding: '14px',
    fontSize: '13px',
    backgroundColor: 'surface.default',
    _dark: { backgroundColor: 'surface.subtle' },
    boxShadow: 'small',
  });
  const headerClass = flex({ alignItems: 'center', gap: '6px', marginBottom: '10px' });
  const titleClass = css({ fontSize: '13px', fontWeight: 'semibold' });
  const actionsClass = flex({ alignItems: 'center', gap: '8px', justifyContent: 'flex-end', marginTop: '12px' });
  const noticeClass = css({ marginRight: 'auto', minWidth: '0', fontSize: '[11.5px]', color: 'text.faint' });
  const tailClass = flex({
    alignItems: 'center',
    gap: '8px',
    marginTop: '12px',
    paddingTop: '10px',
    borderTopWidth: '1px',
    borderColor: 'border.subtle',
    fontSize: '[12.5px]',
    fontWeight: 'semibold',
    color: 'text.subtle',
  });
</script>

{#if entry}
  {@const Body = entry.body}
  <div class={cardClass}>
    <div class={headerClass}>
      <Icon style={css.raw({ flexShrink: '0', color: 'text.subtle' })} icon={entry.icon} size={14} />
      <p class={titleClass}>{entry.title}</p>
    </div>

    <Body input={message.data} onReady={setReady} result={message.result} />

    {#if open}
      <div class={actionsClass}>
        {#if sendFailed}
          <span class={noticeClass}>요청을 보내지 못했어요. 잠시 후 다시 시도해 주세요</span>
        {/if}

        <Button disabled={busy} onclick={() => void submit(false)} size="sm" variant="ghost">그대로 두기</Button>
        <Button disabled={busy || !ready} onclick={() => void submit(true)} size="sm" variant={entry.action}>{entry.confirmLabel}</Button>
      </div>
    {:else}
      <div class={tailClass}>
        <span>{actionTail(outcome, entry)}</span>
      </div>
    {/if}
  </div>
{/if}
