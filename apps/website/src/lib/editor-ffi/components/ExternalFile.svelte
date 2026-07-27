<script lang="ts">
  import { flip, hide } from '@floating-ui/dom';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem, RingSpinner } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import DownloadIcon from '~icons/lucide/download';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import FileIcon from '~icons/lucide/file';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { formatFileSize } from '$lib/utils/format';
  import { getEditorContext } from '../editor.svelte';
  import { attachmentStage, isEmptyLikeStage } from './attachment-stage';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';

  type Props = {
    element: ExternalElement;
  };

  let { element }: Props = $props();

  const ctx = getEditorContext();

  const fileData = $derived(element.data.type === 'file' ? element.data : undefined);
  const fileId = $derived(fileData?.id || undefined);
  const asset = $derived(ctx.editor?.asset(fileId, 'file'));
  const localUpload = $derived(fileId ? ctx.editor?.localUploads.get(fileId) : undefined);
  const resolution = $derived(fileId ? ctx.editor?.resolutions.get(fileId) : undefined);
  const serverPending = $derived(resolution?.state === 'pending' ? resolution.meta : undefined);
  const unresolved = $derived(resolution?.state === 'missing');
  const stage = $derived(attachmentStage({ hasAsset: !!asset, localPhase: localUpload?.phase, hasId: !!fileId, unresolved }));
  // 실패 카드는 자체 affordance(재시도·다시 선택·삭제)를 가지므로 빈 카드 취급에서 제외한다.
  const isEmptyLike = $derived(isEmptyLikeStage(stage));
  const pendingLabel = $derived(serverPending?.name || '파일을 불러오는 중...');

  const canEdit = $derived(!ctx.editor?.readOnly);
  const hasFile = $derived(!!asset || stage === 'localActive');
  const displayName = $derived(asset?.name || localUpload?.name || '파일');
  const displaySize = $derived(asset ? formatFileSize(Number(asset.size)) : undefined);
  const selectedBlockNodes = $derived(ctx.editor?.blockState?.nodes ?? []);
  const isOnlySelectedElement = $derived(
    element.is_selected && selectedBlockNodes.length === 1 && selectedBlockNodes[0]?.id === element.node,
  );
  const isAttachmentDropTarget = $derived(isEmptyLike && ctx.attachmentDropTargetNodeId === element.node);

  let pickerOpened = $state(false);

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 4,
    middleware: [flip(), hide()],
  });

  $effect(() => {
    // 픽커는 `empty`와 `unresolved`에서 열린다 — 후자는 같은 ID를 재예약해 이어받는다(문서 무변경).
    // 아직 해석 중인 `serverPending` 위로는 띄우지 않는다(클릭 한 번에 id가 교체된다).
    pickerOpened = isOnlySelectedElement && isEmptyLike;
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

  const onFailure = ({ file }: { file: File }) => {
    Toast.error(`${file.name} 파일 업로드에 실패했습니다.`);
  };

  const handleUpload = () => {
    const editor = ctx.editor;
    if (!editor || editor.readOnly || stage === 'serverPending') return;
    const nodeId = element.node;
    // 실패한 로컬 시도가 남아 있으면 그 시도를 접고 같은 ID를 다시 잡는다(다시 선택).
    // 미해석 노드는 빈 placeholder와 똑같이 다룬다 — importer가 문서에 적힌 ID를 그대로 재예약한다.
    const reselectId = localUpload ? fileId : undefined;

    const picker = document.createElement('input');
    picker.type = 'file';
    picker.multiple = reselectId === undefined;

    picker.addEventListener('change', () => {
      if (ctx.editor !== editor || editor.destroyed || editor.readOnly) return;
      const files = [...(picker.files ?? [])];

      if (reselectId !== undefined) {
        const file = files[0];
        if (!file) return;
        if (ctx.attachmentImporter.reselect({ assetId: reselectId, nodeId, kind: 'file', file, onFailure })) {
          editor.focus();
        }
        return;
      }

      const items = files.map((file) => ({ file, kind: 'file' as const }));
      if (ctx.attachmentImporter.importAtSelection(items, { existingNodeId: nodeId, onFailure })) {
        editor.focus();
      }
    });

    picker.click();
  };

  const handleRetry = () => {
    if (fileId) ctx.attachmentImporter.retry(fileId);
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

      {#if stage === 'localActive'}
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
        {#if stage === 'serverPending'}
          {pendingLabel}
        {:else if stage === 'localFailed'}
          파일 업로드에 실패했습니다
        {:else if isAttachmentDropTarget}
          놓아서 업로드하기
        {:else}
          파일
        {/if}
      </div>

      {#if stage === 'serverPending'}
        <div class={css({ marginRight: '14px' })}>
          <RingSpinner style={css.raw({ size: '16px', color: 'text.disabled' })} />
        </div>
      {:else if stage === 'localFailed' && canEdit}
        <div class={flex({ alignItems: 'center', gap: '4px', marginRight: '12px' })}>
          {#each [{ label: '재시도', onclick: handleRetry }, { label: '다시 선택', onclick: handleUpload }] as action (action.label)}
            <button
              class={css({
                borderRadius: '4px',
                paddingX: '8px',
                paddingY: '4px',
                fontSize: '13px',
                color: 'text.muted',
                _hover: { backgroundColor: 'interactive.hover', color: 'text.default' },
              })}
              onclick={action.onclick}
              onpointerdown={(e) => {
                e.preventDefault();
                e.stopPropagation();
              }}
              type="button"
            >
              {action.label}
            </button>
          {/each}

          <button
            class={center({
              borderRadius: '4px',
              padding: '4px',
              color: 'text.disabled',
              _hover: { backgroundColor: 'interactive.hover', color: 'text.danger' },
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
