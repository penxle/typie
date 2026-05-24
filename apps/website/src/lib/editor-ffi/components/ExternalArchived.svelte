<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import ArchiveIcon from '~icons/lucide/archive';
  import { getEditorContext } from '../editor.svelte';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';

  type Props = {
    element: ExternalElement;
  };

  let { element }: Props = $props();

  const ctx = getEditorContext();

  const archivedData = $derived(element.data.type === 'archived' ? element.data : undefined);
  const archivedId = $derived(archivedData?.id || undefined);
  const asset = $derived(archivedId ? ctx.editor?.archivedAssets.get(archivedId) : undefined);
  const canEdit = $derived(!ctx.editor?.readOnly);
  const hasPreviewButton = $derived(canEdit && !!asset?.content);

  let open = $state(false);
</script>

<ExternalElementWrapper {element}>
  <div class={css({ width: 'full' })}>
    <div
      class={flex({
        justifyContent: 'space-between',
        alignItems: 'center',
        borderRadius: '4px',
        backgroundColor: 'surface.muted',
        width: 'full',
        height: '48px',
      })}
    >
      <div
        class={flex({
          align: 'center',
          gap: '12px',
          paddingX: '14px',
          paddingY: '12px',
          fontSize: '14px',
          color: 'text.disabled',
          flex: '[1 999 0px]',
          minWidth: '0',
        })}
      >
        <Icon class={css({ flexShrink: '0' })} icon={ArchiveIcon} size={20} />
        <span
          class={css({
            flex: '1',
            minWidth: '0',
            display: 'block',
            overflow: 'hidden',
            whiteSpace: 'nowrap',
            textOverflow: 'ellipsis',
          })}
        >
          보관된 블록
        </span>
      </div>

      {#if hasPreviewButton}
        <div
          class={css({
            marginRight: '12px',
            display: 'flex',
            justifyContent: 'flex-end',
            flex: '[0 1 96px]',
            minWidth: '0',
            overflow: 'hidden',
          })}
        >
          <Button
            style={css.raw({ width: 'full', maxWidth: 'full', minWidth: '0', overflow: 'hidden' })}
            onclick={() => {
              open = true;
            }}
            onpointerdown={(event: PointerEvent) => {
              event.stopPropagation();
            }}
            size="sm"
            variant="secondary"
          >
            <span
              class={css({
                display: 'block',
                overflow: 'hidden',
                whiteSpace: 'nowrap',
                textOverflow: 'ellipsis',
              })}
            >
              내용 보기
            </span>
          </Button>
        </div>
      {/if}
    </div>
  </div>
</ExternalElementWrapper>

{#if asset?.content}
  <Modal style={css.raw({ maxWidth: '560px' })} bind:open>
    <div class={css({ padding: '24px' })}>
      <div class={flex({ justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' })}>
        <h2
          class={css({
            fontSize: '16px',
            fontWeight: 'semibold',
            color: 'text.default',
          })}
        >
          보관된 블록
        </h2>
        <Button
          onclick={async () => {
            await navigator.clipboard.writeText(asset.content ?? '');
          }}
          size="sm"
          variant="secondary"
        >
          복사
        </Button>
      </div>

      <div
        class={css({
          maxHeight: '400px',
          overflowY: 'auto',
          padding: '12px',
          borderRadius: '6px',
          backgroundColor: 'surface.subtle',
          '& pre': {
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-all',
            padding: '12px',
            borderRadius: '6px',
            backgroundColor: 'surface.muted',
            fontSize: '13px',
            fontFamily: 'mono',
            color: 'text.subtle',
          },
          '& pre code': {
            fontFamily: '[inherit]',
            fontSize: '[inherit]',
          },
          '& p': {
            marginY: '0',
          },
          '& table': {
            width: 'full',
            borderCollapse: 'collapse',
          },
          '& td': {
            border: '1px solid',
            borderColor: 'border.default',
            padding: '8px',
          },
          '& img': {
            maxWidth: 'full',
          },
          '& a': {
            color: 'accent.brand.default',
            textDecoration: 'underline',
          },
        })}
      >
        <!-- eslint-disable-next-line svelte/no-at-html-tags -->
        {@html asset.content}
      </div>

      <div class={flex({ justifyContent: 'flex-end', marginTop: '16px' })}>
        <Button
          onclick={() => {
            open = false;
          }}
          size="sm"
          variant="secondary"
        >
          닫기
        </Button>
      </div>
    </div>
  </Modal>
{/if}
