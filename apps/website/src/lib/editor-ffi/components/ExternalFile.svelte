<script lang="ts">
  import { flip, hide } from '@floating-ui/dom';
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem, RingSpinner } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import DownloadIcon from '~icons/lucide/download';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FileIcon from '~icons/lucide/file';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { formatFileSize } from '$lib/utils/format';
  import { getEditorContext } from '../editor.svelte';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';

  type Props = {
    element: ExternalElement;
  };

  let { element }: Props = $props();

  const ctx = getEditorContext();

  const fileData = $derived(element.data.type === 'file' ? element.data : undefined);
  const fileId = $derived(fileData?.id || undefined);
  const asset = $derived(fileId ? ctx.fileAssets.get(fileId) : undefined);
  const inflight = $derived(ctx.editor?.inflightFiles.get(element.node));
  const stage = $derived.by(() => {
    if (asset) return 'ready';
    if (inflight) return 'uploading';
    if (fileId) return 'resolving';
    return 'empty';
  });

  const canEdit = $derived(!ctx.editor?.readOnly);
  const hasFile = $derived(!!asset || stage === 'uploading');
  const displayName = $derived(asset?.name || inflight?.name || '파일');
  const displaySize = $derived(asset ? formatFileSize(Number(asset.size)) : undefined);
  const selectedBlockNodes = $derived(ctx.editor?.blockState?.nodes ?? []);
  const isOnlySelectedElement = $derived(
    element.is_selected && selectedBlockNodes.length === 1 && selectedBlockNodes[0]?.id === element.node,
  );
  const isAttachmentDropTarget = $derived(stage === 'empty' && ctx.attachmentDropTargetNodeId === element.node);

  let pickerOpened = $state(false);

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 4,
    middleware: [flip(), hide()],
  });

  $effect(() => {
    pickerOpened = isOnlySelectedElement && stage === 'empty';
  });

  const deleteNode = () => {
    const editor = ctx.editor;
    if (!editor) return;

    ctx.attachmentImporter.cancelNode(editor, element.node);
    editor.enqueue({
      type: 'node',
      op: { type: 'delete', id: element.node },
    });
    editor.focus();
  };

  const handleUpload = () => {
    const editor = ctx.editor;
    if (!editor || editor.readOnly) return;
    const nodeId = element.node;

    const picker = document.createElement('input');
    picker.type = 'file';
    picker.multiple = true;

    picker.addEventListener('change', () => {
      if (ctx.editor !== editor || editor.destroyed || editor.readOnly) return;
      const items = [...(picker.files ?? [])].map((file) => ({ file, kind: 'file' as const }));
      if (
        ctx.attachmentImporter.importAtSelection(items, {
          existingNodeId: nodeId,
          onFailure: ({ file }) => {
            Toast.error(`${file.name} 파일 업로드에 실패했습니다.`);
          },
        })
      ) {
        editor.focus();
      }
    });

    picker.click();
  };

  const handleDownload = () => {
    if (!asset) return;
    const a = document.createElement('a');
    a.href = asset.url;
    a.download = asset.name;
    a.click();
  };
</script>

<ExternalElementWrapper {element}>
  {#if hasFile}
    <div
      class={cx(
        'group',
        flex({
          alignItems: 'center',
          gap: '12px',
          borderWidth: '1px',
          borderColor: 'border.subtle',
          borderRadius: '8px',
          paddingX: '16px',
          paddingY: '12px',
          backgroundColor: 'surface.muted',
          transition: 'common',
          _hover: { borderColor: 'border.default' },
        }),
      )}
    >
      <Icon class={css({ color: 'text.muted', flexShrink: '0' })} icon={FileIcon} size={20} />

      <div class={flex({ direction: 'column', flex: '1', minWidth: '0' })}>
        <span
          class={css({
            fontSize: '14px',
            fontWeight: 'medium',
            color: 'text.default',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          })}
        >
          {displayName}
        </span>
        {#if displaySize}
          <span class={css({ fontSize: '12px', color: 'text.muted' })}>
            {displaySize}
          </span>
        {/if}
      </div>

      {#if canEdit}
        <button
          class={css({
            padding: '4px',
            borderRadius: '4px',
            color: 'text.muted',
            opacity: '0',
            transition: 'common',
            _hover: { backgroundColor: 'interactive.hover', color: 'text.danger' },
            _groupHover: { opacity: '100' },
          })}
          aria-label="파일 삭제"
          onclick={deleteNode}
          onpointerdown={(e) => {
            e.preventDefault();
            e.stopPropagation();
          }}
          type="button"
        >
          <Icon icon={Trash2Icon} size={16} />
        </button>
      {/if}

      {#if stage === 'uploading'}
        <RingSpinner style={css.raw({ size: '20px', color: 'text.disabled' })} />
      {:else if asset}
        <button
          class={css({
            padding: '4px',
            borderRadius: '4px',
            color: 'text.muted',
            transition: 'common',
            _hover: { backgroundColor: 'interactive.hover', color: 'text.default' },
          })}
          aria-label="파일 다운로드"
          onclick={handleDownload}
          onpointerdown={(e) => {
            e.preventDefault();
            e.stopPropagation();
          }}
          type="button"
        >
          <Icon icon={DownloadIcon} size={16} />
        </button>
      {/if}
    </div>
  {:else}
    <div
      class={cx(
        'group',
        flex({
          justifyContent: 'space-between',
          alignItems: 'center',
          borderRadius: '4px',
          backgroundColor: 'surface.muted',
          width: 'full',
          height: '48px',
        }),
        isAttachmentDropTarget && css({ boxShadow: '[inset 0 0 0 1px token(colors.palette.blue)]' }),
      )}
      use:anchor
    >
      <div
        class={flex({
          align: 'center',
          gap: '12px',
          paddingX: '14px',
          paddingY: '12px',
          fontSize: '14px',
          color: isAttachmentDropTarget ? 'palette.blue' : 'text.disabled',
        })}
      >
        <Icon icon={FileIcon} size={20} />
        {stage === 'resolving' ? '파일을 불러오는 중...' : isAttachmentDropTarget ? '놓아서 업로드하기' : '파일'}
      </div>

      {#if stage === 'resolving'}
        <div class={css({ marginRight: '14px' })}>
          <RingSpinner style={css.raw({ size: '16px', color: 'text.disabled' })} />
        </div>
      {:else if canEdit && !isAttachmentDropTarget}
        <div
          onpointerdown={(e) => {
            e.stopPropagation();
          }}
          role="none"
        >
          <Menu>
            {#snippet button({ open }: { open: boolean })}
              <div
                class={css(
                  {
                    marginRight: '12px',
                    borderRadius: '4px',
                    padding: '2px',
                    color: 'text.disabled',
                    opacity: '0',
                    transition: 'common',
                    _hover: { backgroundColor: 'interactive.hover' },
                    _groupHover: { opacity: '100' },
                  },
                  open && { opacity: '100' },
                )}
              >
                <Icon icon={EllipsisIcon} size={20} />
              </div>
            {/snippet}

            <MenuItem onclick={deleteNode} variant="danger">
              <Icon icon={Trash2Icon} size={12} />
              <span>삭제</span>
            </MenuItem>
          </Menu>
        </div>
      {/if}
    </div>
  {/if}
</ExternalElementWrapper>

{#if pickerOpened && canEdit}
  <button
    class={flex({
      alignItems: 'center',
      gap: '6px',
      borderWidth: '1px',
      borderRadius: '8px',
      paddingX: '12px',
      paddingY: '6px',
      fontSize: '13px',
      color: 'text.muted',
      backgroundColor: 'surface.default',
      boxShadow: 'small',
      transition: 'common',
      zIndex: 'editor',
      _hover: { backgroundColor: 'interactive.hover' },
    })}
    onclick={handleUpload}
    type="button"
    use:floating
  >
    <Icon icon={FileIcon} size={14} />
    파일 선택
  </button>
{/if}
