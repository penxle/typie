<script lang="ts">
  import { flip, hide } from '@floating-ui/dom';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { createFloatingActions, pointerCapture } from '@typie/ui/actions';
  import { Icon, Img, RingSpinner } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import DownloadIcon from '~icons/lucide/download';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import ImageIcon from '~icons/lucide/image';
  import Maximize2Icon from '~icons/lucide/maximize-2';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { getEditorContext } from '../editor.svelte';
  import { calculateImageContainerSize, calculateImageWidth } from '../handlers/image';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import ExternalImageEnlarge from './ExternalImageEnlarge.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';

  type Props = {
    element: ExternalElement;
  };

  type ResizeSession = {
    x: number;
    width: number;
    proportion: number;
    reverse: boolean;
    boundsWidth: number;
  };

  let { element }: Props = $props();

  const ctx = getEditorContext();

  let proportion = $state(100);
  let isResizing = $state(false);
  let enlarged = $state(false);
  let containerEl = $state<HTMLDivElement>();
  let pickerOpened = $state(false);

  const imageData = $derived(element.data.type === 'image' ? element.data : undefined);
  const imageId = $derived(imageData?.id || undefined);
  const asset = $derived(imageId ? ctx.editor?.imageAssets.get(imageId) : undefined);
  const inflight = $derived(ctx.editor?.inflightImages.get(element.node));
  const stage = $derived.by(() => {
    if (asset) return 'ready';
    if (inflight) return 'uploading';
    if (imageId) return 'resolving';
    return 'empty';
  });

  const imageSrc = $derived(asset?.url ?? inflight?.url);
  const originalWidth = $derived(asset?.width ?? inflight?.width ?? 0);
  const originalHeight = $derived(asset?.height ?? inflight?.height ?? 0);
  const liveWidth = $derived(calculateImageWidth(element.bounds.width, proportion, originalWidth));
  const containerSize = $derived(
    calculateImageContainerSize({
      boundsWidth: element.bounds.width,
      proportion,
      originalWidth,
      originalHeight,
    }),
  );
  const canEdit = $derived(!ctx.editor?.readOnly);
  const selectedBlockNodes = $derived(ctx.editor?.blockState?.nodes ?? []);
  const isOnlySelectedElement = $derived(
    element.is_selected && selectedBlockNodes.length === 1 && selectedBlockNodes[0]?.id === element.node,
  );
  const isAttachmentDropTarget = $derived(stage === 'empty' && ctx.attachmentDropTargetNodeId === element.node);

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 4,
    middleware: [flip(), hide()],
  });

  $effect(() => {
    if (imageData && !isResizing) {
      proportion = imageData.proportion;
    }
  });

  $effect(() => {
    pickerOpened = isOnlySelectedElement && stage === 'empty';
  });

  $effect(() => {
    if (stage !== 'ready') {
      enlarged = false;
    }
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
    picker.accept = 'image/*';
    picker.multiple = true;
    picker.addEventListener('change', () => {
      if (ctx.editor !== editor || editor.destroyed || editor.readOnly) return;
      const items = [...(picker.files ?? [])].map((file) => ({ file, kind: 'image' as const }));
      if (
        ctx.attachmentImporter.importAtSelection(items, {
          existingNodeId: nodeId,
          onFailure: ({ file }) => {
            Toast.error(`${file.name} 이미지 업로드에 실패했습니다.`);
          },
        })
      ) {
        editor.focus();
      }
    });
    picker.click();
  };

  const getWidthBounds = (boundsWidth: number) => {
    const maxWidth = originalWidth > 0 ? Math.min(originalWidth, boundsWidth) : boundsWidth;
    const minWidth = Math.min(maxWidth, Math.max(boundsWidth * 0.1, 100));
    return { minWidth, maxWidth };
  };

  const clampWidth = (width: number, boundsWidth: number) => {
    const { minWidth, maxWidth } = getWidthBounds(boundsWidth);
    return Math.max(minWidth, Math.min(maxWidth, width));
  };

  const handleResizeStart = (event: PointerEvent, reverse: boolean): ResizeSession | null => {
    if (isResizing || !event.isPrimary || event.button !== 0) return null;

    event.preventDefault();
    event.stopPropagation();

    isResizing = true;
    return {
      x: event.clientX,
      width: liveWidth,
      proportion,
      reverse,
      boundsWidth: element.bounds.width,
    };
  };

  const handleResize = (session: ResizeSession, event: PointerEvent) => {
    const { boundsWidth } = session;
    if (boundsWidth <= 0) return;

    const dx = (ctx.editor?.clientDeltaToLocalDelta(event.clientX - session.x) ?? event.clientX - session.x) * (session.reverse ? -1 : 1);
    const newWidth = clampWidth(session.width + dx * 2, boundsWidth);
    proportion = (newWidth / boundsWidth) * 100;
  };

  const handleResizeEnd = (session: ResizeSession, event: PointerEvent) => {
    handleResize(session, event);
    const finalProportion = Math.round(proportion);
    isResizing = false;
    ctx.editor?.enqueue({
      type: 'node',
      op: {
        type: 'set_attr',
        id: element.node,
        attr: {
          type: 'image',
          attr: { type: 'proportion', value: finalProportion },
        },
      },
    });
    ctx.editor?.focus();
  };

  const handleResizeCancel = (session: ResizeSession) => {
    proportion = session.proportion;
    isResizing = false;
  };

  const handleOpenInNewTab = () => {
    const url = asset?.originalUrl;
    if (!url) return;
    window.open(url, '_blank');
  };

  const handleSaveAs = async () => {
    const url = asset?.originalUrl;
    if (!url) return;

    try {
      const resp = await fetch(url);
      const blob = await resp.blob();
      const disposition = resp.headers.get('content-disposition');
      const starMatch = disposition?.match(/filename\*=UTF-8''(.+?)(?:;|$)/);
      const quotedMatch = disposition?.match(/filename="(.+?)"/);
      const rawFilename = starMatch?.[1] ?? quotedMatch?.[1];
      const filename = rawFilename ? decodeURIComponent(rawFilename) : `image.${blob.type.split('/')[1] ?? 'png'}`;
      const a = document.createElement('a');
      a.href = URL.createObjectURL(blob);
      a.download = filename;
      a.click();
      URL.revokeObjectURL(a.href);
    } catch {
      Toast.error('이미지 저장에 실패했습니다.');
    }
  };

  $effect(() => {
    const editor = ctx.editor;
    const el = containerEl;
    if (!editor || !el) return;

    return editor.registerContextMenuContributor(({ clientX, clientY }) => {
      if (!asset) return [];
      const rect = el.getBoundingClientRect();
      if (clientX < rect.left || clientX > rect.right || clientY < rect.top || clientY > rect.bottom) {
        return [];
      }
      return [
        { label: '이미지 내려받기', icon: DownloadIcon, onclick: () => void handleSaveAs() },
        { label: '새 탭에서 이미지 열기', icon: ExternalLinkIcon, onclick: handleOpenInNewTab },
      ];
    });
  });
</script>

<ExternalElementWrapper {element} minHeight={stage === 'ready' ? undefined : '48px'}>
  <div
    bind:this={containerEl}
    style:width={containerSize.width}
    style:height={containerSize.height}
    class={cx('group', css({ position: 'relative', margin: '[0 auto]' }))}
    role="group"
  >
    {#if imageSrc}
      <Img
        style={css.raw({ width: 'full', borderRadius: '4px' }, !canEdit && { cursor: 'zoom-in' })}
        alt="본문 이미지"
        aria-label={canEdit ? undefined : '이미지 확대 보기'}
        onclick={() => {
          if (!canEdit) enlarged = true;
        }}
        onkeydown={(event) => {
          if (canEdit || !(event.key === 'Enter' || event.key === ' ')) {
            return;
          }

          event.preventDefault();
          enlarged = true;
        }}
        onpointerdown={(event) => {
          if (!canEdit) event.stopPropagation();
        }}
        placeholder={asset?.placeholder}
        progressive
        ratio={originalHeight > 0 ? originalWidth / originalHeight : undefined}
        role={canEdit ? undefined : 'button'}
        size="full"
        tabindex={canEdit ? undefined : 0}
        url={imageSrc}
      />

      {#if stage === 'uploading'}
        <div class={center({ position: 'absolute', inset: '0', backgroundColor: 'white/50' })}>
          <RingSpinner style={css.raw({ size: '24px', color: 'text.disabled' })} />
        </div>
      {/if}

      {#if canEdit && stage === 'ready'}
        <div class={flex({ position: 'absolute', top: '10px', right: '10px', gap: '6px', zIndex: '10' })}>
          <button
            class={center({
              borderRadius: '4px',
              size: '28px',
              color: 'text.bright',
              backgroundColor: '[#363839/70]',
              opacity: '0',
              transition: 'opacity',
              _hover: { backgroundColor: '[#363839/40]' },
              _groupHover: { opacity: '100' },
            })}
            aria-label="이미지 확대 보기"
            onclick={() => (enlarged = true)}
            onpointerdown={(event) => {
              event.stopPropagation();
            }}
            type="button"
          >
            <Icon icon={Maximize2Icon} size={16} />
          </button>

          <button
            class={center({
              borderRadius: '4px',
              size: '28px',
              color: 'text.bright',
              backgroundColor: '[#363839/70]',
              opacity: '0',
              transition: 'opacity',
              _hover: { backgroundColor: '[#363839/40]' },
              _groupHover: { opacity: '100' },
            })}
            aria-label="이미지 삭제"
            onclick={deleteNode}
            onpointerdown={(event) => {
              event.stopPropagation();
            }}
            type="button"
          >
            <Icon icon={Trash2Icon} size={16} />
          </button>
        </div>

        <div class={flex({ position: 'absolute', top: '0', bottom: '0', left: '10px', alignItems: 'center', pointerEvents: 'none' })}>
          <button
            class={css({
              borderRadius: '4px',
              backgroundColor: 'white/50',
              mixBlendMode: 'difference',
              width: '8px',
              height: '1/3',
              maxHeight: '72px',
              cursor: 'col-resize',
              opacity: '0',
              transition: 'opacity',
              zIndex: '10',
              pointerEvents: 'auto',
              _hover: { backgroundColor: 'white/40' },
              _groupHover: { opacity: '100' },
            })}
            aria-label="이미지 크기 조절"
            type="button"
            use:pointerCapture={{
              start: (event) => handleResizeStart(event, true),
              move: handleResize,
              end: handleResizeEnd,
              cancel: handleResizeCancel,
            }}
          ></button>
        </div>

        <div class={flex({ position: 'absolute', top: '0', bottom: '0', right: '10px', alignItems: 'center', pointerEvents: 'none' })}>
          <button
            class={css({
              borderRadius: '4px',
              backgroundColor: 'white/50',
              mixBlendMode: 'difference',
              width: '8px',
              height: '1/3',
              maxHeight: '72px',
              cursor: 'col-resize',
              opacity: '0',
              transition: 'opacity',
              zIndex: '10',
              pointerEvents: 'auto',
              _hover: { backgroundColor: 'white/40' },
              _groupHover: { opacity: '100' },
            })}
            aria-label="이미지 크기 조절"
            type="button"
            use:pointerCapture={{
              start: (event) => handleResizeStart(event, false),
              move: handleResize,
              end: handleResizeEnd,
              cancel: handleResizeCancel,
            }}
          ></button>
        </div>
      {/if}
    {:else}
      <div
        class={cx(
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
          <Icon icon={ImageIcon} size={20} />
          {#if stage === 'resolving'}
            이미지를 불러오는 중...
          {:else if stage === 'uploading'}
            이미지를 업로드하는 중...
          {:else if isAttachmentDropTarget}
            놓아서 업로드하기
          {:else}
            이미지
          {/if}
        </div>

        {#if stage === 'resolving' || stage === 'uploading'}
          <div class={css({ marginRight: '14px' })}>
            <RingSpinner style={css.raw({ size: '16px', color: 'text.disabled' })} />
          </div>
        {:else if canEdit && !isAttachmentDropTarget}
          <button
            class={center({
              marginRight: '12px',
              borderRadius: '4px',
              padding: '4px',
              color: 'text.disabled',
              _hover: { backgroundColor: 'interactive.hover', color: 'text.danger' },
            })}
            aria-label="이미지 삭제"
            onclick={deleteNode}
            onpointerdown={(event) => {
              event.preventDefault();
              event.stopPropagation();
            }}
            type="button"
          >
            <Icon icon={Trash2Icon} size={16} />
          </button>
        {/if}
      </div>
    {/if}
  </div>
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
    <Icon icon={ImageIcon} size={14} />
    이미지 선택
  </button>
{/if}

{#if enlarged && stage === 'ready' && imageSrc && containerEl}
  <ExternalImageEnlarge
    onclose={() => (enlarged = false)}
    placeholder={asset?.placeholder}
    ratio={originalHeight > 0 ? originalWidth / originalHeight : undefined}
    referenceEl={containerEl}
    url={imageSrc}
  />
{/if}
