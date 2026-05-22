<script lang="ts">
  import { flip, hide } from '@floating-ui/dom';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon, Img, RingSpinner } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import ImageIcon from '~icons/lucide/image';
  import Maximize2Icon from '~icons/lucide/maximize-2';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { getEditorContext } from '../editor.svelte';
  import {
    calculateImageHeight,
    calculateImageWidth,
    createDeleteNodeMessage,
    createSetImageAttrsMessage,
    getFirstImageFile,
    processImageUpload,
  } from '../handlers/image-flow';
  import { getImageDimensions, uploadImageFile } from '../handlers/upload';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import ExternalImageEnlarge from './ExternalImageEnlarge.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';

  type Props = {
    element: ExternalElement;
  };

  let { element }: Props = $props();

  const ctx = getEditorContext();

  let pickerOpened = $state(false);
  let proportion = $state(100);
  let isResizing = $state(false);
  let initialResizeData: { x: number; width: number; proportion: number; reverse: boolean } | null = null;
  let enlarged = $state(false);
  let containerEl = $state<HTMLDivElement>();

  const imageData = $derived(element.data.type === 'image' ? element.data : undefined);
  const asset = $derived(imageData?.id ? ctx.editor?.imageAssets.get(imageData.id) : undefined);
  const inflight = $derived(ctx.editor?.inflightImages.get(element.node_id));
  const imageSrc = $derived(asset?.url ?? inflight?.url);
  const hasImage = $derived(!!imageSrc);
  const isUploading = $derived(!!inflight && !asset);
  const isResolvingAsset = $derived(!!imageData?.id && !asset && !inflight);
  const originalWidth = $derived(asset?.width ?? inflight?.width ?? 0);
  const originalHeight = $derived(asset?.height ?? inflight?.height ?? 0);
  const liveWidth = $derived(calculateImageWidth(element.bounds.width, proportion, originalWidth));
  const liveHeight = $derived(calculateImageHeight(liveWidth, originalWidth, originalHeight));
  const canEdit = $derived(!ctx.editor?.readOnly);

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
    pickerOpened = element.is_selected && !hasImage && !isResolvingAsset;
  });

  $effect(() => {
    if (!hasImage) {
      enlarged = false;
    }
  });

  $effect(() => {
    if (asset && inflight) {
      deleteInflightImage(element.node_id);
      revokeObjectUrl(inflight.url);
    }
  });

  const enqueueImageAttrs = (id: string | undefined, nextProportion: number) => {
    ctx.editor?.enqueue(createSetImageAttrsMessage(element.node_id, id, nextProportion));
  };

  const deleteInflightImage = (nodeId: string) => {
    ctx.editor?.inflightImages.delete(nodeId);
  };

  const revokeObjectUrl = (url: string) => {
    URL.revokeObjectURL(url);
  };

  const deleteNode = () => {
    ctx.editor?.enqueue(createDeleteNodeMessage(element.node_id));
    ctx.editor?.focus();
  };

  const processFile = async (file: File) => {
    const editor = ctx.editor;
    if (!editor || !imageData) return;

    const result = await processImageUpload({
      file,
      nodeId: element.node_id,
      getProportion: () => proportion,
      setInflightImage: (nodeId, image) => editor.inflightImages.set(nodeId, image),
      deleteInflightImage,
      setImageAsset: (asset) => editor.imageAssets.set(asset.id, asset),
      enqueue: (message) => editor.enqueue(message),
      focus: () => editor.focus(),
      createObjectUrl: (file) => URL.createObjectURL(file),
      revokeObjectUrl,
      readImageDimensions: getImageDimensions,
      uploadImageFile,
    });

    if (result === 'failed') {
      Toast.error(`${file.name} 이미지 업로드에 실패했습니다.`);
    }
  };

  const handleUpload = () => {
    if (!canEdit) return;
    // TODO: restrictedBlob 용량 제한 처리.

    const picker = document.createElement('input');
    picker.type = 'file';
    picker.accept = 'image/*';

    picker.addEventListener('change', () => {
      pickerOpened = false;

      const file = picker.files?.[0];
      if (!file) {
        deleteNode();
        return;
      }

      void processFile(file);
    });

    picker.addEventListener('cancel', () => {
      pickerOpened = false;
      deleteNode();
    });

    picker.click();
  };

  const handleDragOver = (event: DragEvent) => {
    if (!canEdit || hasImage) return;

    const items = [...(event.dataTransfer?.items ?? [])];
    if (items.length > 0 && !items.some((item) => item.kind === 'file' && item.type.startsWith('image/'))) return;

    event.preventDefault();

    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'copy';
    }
  };

  const handleDrop = (event: DragEvent) => {
    if (!canEdit || hasImage) return;

    event.preventDefault();

    const file = getFirstImageFile(event.dataTransfer?.files ?? []);
    if (!file) return;

    pickerOpened = false;
    void processFile(file);
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

  const handleResizeStart = (event: PointerEvent, reverse: boolean) => {
    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture(event.pointerId);
    event.preventDefault();

    isResizing = true;
    initialResizeData = {
      x: event.clientX,
      width: liveWidth,
      proportion,
      reverse,
    };
  };

  const handleResize = (event: PointerEvent) => {
    const target = event.currentTarget as HTMLElement;
    if (!target.hasPointerCapture(event.pointerId) || !initialResizeData) return;

    const boundsWidth = element.bounds.width;
    if (boundsWidth <= 0) return;

    const dx = (event.clientX - initialResizeData.x) * (initialResizeData.reverse ? -1 : 1);
    const newWidth = clampWidth(initialResizeData.width + dx * 2, boundsWidth);
    proportion = (newWidth / boundsWidth) * 100;
  };

  const handleResizeEnd = (event: PointerEvent) => {
    const target = event.currentTarget as HTMLElement;
    if (target.hasPointerCapture(event.pointerId)) {
      target.releasePointerCapture(event.pointerId);
    }

    // do NOT set isResizing = false here
    enqueueImageAttrs(imageData?.id, proportion);
    ctx.editor?.focus();
  };

  // Clear guard once editor state reflects the resize
  $effect(() => {
    if (isResizing && imageData && imageData.proportion === Math.round(proportion)) {
      isResizing = false;
    }
  });
</script>

<ExternalElementWrapper {element} minHeight={hasImage ? undefined : '48px'}>
  <div
    bind:this={containerEl}
    style:width={hasImage ? `${liveWidth}px` : '100%'}
    style:height={hasImage ? `${liveHeight}px` : undefined}
    class={cx('group', css({ position: 'relative', margin: '[0 auto]' }))}
    ondragovercapture={handleDragOver}
    ondropcapture={handleDrop}
    role="group"
  >
    {#if hasImage}
      <Img
        style={css.raw({ width: 'full', borderRadius: '4px' }, !canEdit && { cursor: 'zoom-in' })}
        alt="본문 이미지"
        aria-label={canEdit ? undefined : '이미지 확대 보기'}
        onclick={() => {
          if (canEdit) {
            return;
          }

          enlarged = true;
        }}
        onkeydown={(event) => {
          if (!canEdit && (event.key === 'Enter' || event.key === ' ')) {
            event.preventDefault();
            enlarged = true;
          }
        }}
        onpointerdown={(event) => {
          if (canEdit) {
            return;
          }

          event.stopPropagation();
        }}
        placeholder={asset?.placeholder}
        progressive
        ratio={originalHeight > 0 ? originalWidth / originalHeight : undefined}
        role={canEdit ? undefined : 'button'}
        size="full"
        tabindex={canEdit ? undefined : 0}
        url={imageSrc ?? ''}
      />

      {#if isUploading}
        <div class={center({ position: 'absolute', inset: '0', backgroundColor: 'white/50' })}>
          <RingSpinner style={css.raw({ size: '24px', color: 'text.disabled' })} />
        </div>
      {/if}

      {#if canEdit}
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
            onpointerdown={(event) => {
              event.preventDefault();
              handleResizeStart(event, true);
            }}
            onpointermove={handleResize}
            onpointerup={handleResizeEnd}
            type="button"
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
            onpointerdown={(event) => {
              event.stopPropagation();
              handleResizeStart(event, false);
            }}
            onpointermove={handleResize}
            onpointerup={handleResizeEnd}
            type="button"
          ></button>
        </div>
      {/if}
    {:else}
      <div
        class={flex({
          justifyContent: 'space-between',
          alignItems: 'center',
          borderRadius: '4px',
          backgroundColor: 'surface.muted',
          width: 'full',
          height: '48px',
        })}
        use:anchor
      >
        <div class={flex({ align: 'center', gap: '12px', paddingX: '14px', paddingY: '12px', fontSize: '14px', color: 'text.disabled' })}>
          <Icon icon={ImageIcon} size={20} />
          {isResolvingAsset ? '이미지를 불러오는 중...' : '이미지'}
        </div>

        {#if isResolvingAsset}
          <div class={css({ marginRight: '14px' })}>
            <RingSpinner style={css.raw({ size: '16px', color: 'text.disabled' })} />
          </div>
        {:else if canEdit}
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

{#if enlarged && hasImage && imageSrc && containerEl}
  <ExternalImageEnlarge
    onclose={() => (enlarged = false)}
    placeholder={asset?.placeholder}
    ratio={originalHeight > 0 ? originalWidth / originalHeight : undefined}
    referenceEl={containerEl}
    url={imageSrc}
  />
{/if}
