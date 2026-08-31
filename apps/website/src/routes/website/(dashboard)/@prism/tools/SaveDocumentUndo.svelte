<script lang="ts">
  import { createMutation, getClient } from '@mearie/svelte';
  import { TypieError } from '@typie/lib/errors';
  import { css } from '@typie/styled-system/css';
  import { Button } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { unwrapError } from '$lib/graphql/error';
  import { graphql } from '$mearie';
  import { undoStateQuery } from '../lib/save-document-undo.ts';
  import type { UndoState } from '../lib/save-document-undo.ts';
  import type { TailActionProps } from './action-cards.ts';

  let { sessionId, toolCallId, doneLabel }: TailActionProps = $props();

  const client = getClient();

  let busy = $state(false);
  let inFlight = $state(false);

  let edit = $state<UndoState | null>(null);

  const load = async (): Promise<UndoState | null> => {
    if (sessionId === null) return null;
    const data = await client.query(undoStateQuery, { sessionId, toolCallId }, { fetchPolicy: 'network-only' });
    edit = data.prismDocumentEdit;
    return edit;
  };

  $effect(() => {
    void load().catch(() => null);
  });

  const [undoPrismDocumentEdit] = createMutation(
    graphql(`
      mutation DashboardLayout_PrismSaveDocumentUndo_UndoPrismDocumentEdit_Mutation($input: UndoPrismDocumentEditInput!) {
        undoPrismDocumentEdit(input: $input) {
          undoable
          undone
          changedAfter
        }
      }
    `),
  );

  const undone = $derived(edit?.undone ?? false);

  const NOT_UNDOABLE_MESSAGE = '이제 되돌릴 수 없어요. 타임라인에서 되돌려 주세요';

  const failureFor = (isUndone: boolean) =>
    isUndone ? '다시 적용하지 못했어요. 잠시 후 다시 시도해 주세요' : '되돌리지 못했어요. 잠시 후 다시 시도해 주세요';

  const copyFor = (isUndone: boolean, changed: boolean) =>
    isUndone
      ? {
          title: '편집을 다시 적용할까요?',
          message: changed ? '되돌린 뒤에 문서가 더 바뀌었어요. 다시 적용하면 그 변경도 함께 사라져요.' : '되돌리기 전 상태로 되돌아가요.',
          actionLabel: '다시 적용',
          success: '편집을 다시 적용했어요',
        }
      : {
          title: '편집을 되돌릴까요?',
          message: changed
            ? '이 저장 뒤에 문서가 더 바뀌었어요. 되돌리면 그 변경도 함께 사라져요.'
            : '프리즘이 저장한 내용을 저장 직전으로 되돌려요.',
          actionLabel: '되돌리기',
          success: '편집을 되돌렸어요',
        };

  const request = async () => {
    if (busy || inFlight || sessionId === null) return;

    busy = true;

    let fresh: UndoState | null;
    try {
      fresh = await load();
    } catch {
      Toast.error(failureFor(undone));
      busy = false;
      return;
    }

    if (!fresh?.undoable) {
      Toast.error(NOT_UNDOABLE_MESSAGE);
      busy = false;
      return;
    }

    const wasUndone = fresh.undone;
    const { title, message, actionLabel, success } = copyFor(fresh.undone, fresh.changedAfter);

    Dialog.confirm({
      title,
      message,
      action: 'primary',
      actionLabel,
      actionHandler: async () => {
        if (inFlight) return false;
        inFlight = true;

        try {
          let data;
          try {
            data = await undoPrismDocumentEdit({ input: { sessionId, toolCallId } });
          } catch (err) {
            const error = unwrapError(err);
            Toast.error(
              error instanceof TypieError && error.code === 'prism_edit_superseded' ? NOT_UNDOABLE_MESSAGE : failureFor(wasUndone),
            );
            return;
          }

          edit = data.undoPrismDocumentEdit;

          Toast.success(success);
          mixpanel.track('undo_prism_document_edit', { redo: wasUndone });
        } finally {
          inFlight = false;
        }
      },
      onclose: () => {
        busy = false;
      },
    });
  };
</script>

<span>{undone ? '되돌렸어요' : doneLabel}</span>

{#if edit?.undoable}
  <Button style={css.raw({ marginLeft: 'auto' })} disabled={busy || inFlight} onclick={request} size="sm" variant="secondary">
    {undone ? '다시 적용' : '되돌리기'}
  </Button>
{/if}
