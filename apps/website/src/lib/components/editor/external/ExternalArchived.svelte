<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import ArchiveIcon from '~icons/lucide/archive';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import type { ExternalElement, ExternalElementData } from '$lib/editor/types';

  type ArchivedData = Extract<ExternalElementData, { type: 'archived' }>;

  type Props = {
    el: ExternalElement;
  };

  let { el }: Props = $props();

  const { editor } = getEditorContext();

  const archivedData = $derived(el.data as ArchivedData);
  const isEditable = $derived(!editor.isReadOnly());
  const asset = $derived(archivedData.id ? editor.archivedAssets.get(archivedData.id) : undefined);
  const hasPreviewButton = $derived(isEditable && !!asset?.content);

  let modalOpen = $state(false);
</script>

<ExternalElementWrapper {el}>
  <div class={css({ position: 'relative', width: 'full' })}>
    <div
      class={flex({
        justifyContent: 'space-between',
        alignItems: 'center',
        borderRadius: '4px',
        backgroundColor: 'surface.muted',
        width: 'full',
        height: '48px',
        minWidth: '0',
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
              modalOpen = true;
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
  <Modal style={css.raw({ maxWidth: '560px' })} bind:open={modalOpen}>
    <div class={css({ padding: '24px' })}>
      <h2
        class={css({
          fontSize: '16px',
          fontWeight: 'semibold',
          marginBottom: '16px',
          color: 'text.default',
        })}
      >
        보관된 블록
      </h2>

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
            modalOpen = false;
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
