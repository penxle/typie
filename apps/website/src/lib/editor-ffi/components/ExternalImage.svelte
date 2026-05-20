<script lang="ts">
  import { flip, hide } from '@floating-ui/dom';
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon, Img, Menu, MenuItem, RingSpinner } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import ImageIcon from '~icons/lucide/image';
  import Trash2Icon from '~icons/lucide/trash-2';
  import ZoomInIcon from '~icons/lucide/zoom-in';
  import ExternalImageEnlarge from '$lib/components/editor/external/ExternalImageEnlarge.svelte';
  import { uploadBlobAsImage } from '$lib/utils/blob.svelte';
  import { graphql } from '$mearie';
  import { getEditorContext } from '../editor.svelte';
  import {
    computeImagePresentation,
    createDeleteImageMessage,
    createSetImageAttrsMessage,
    processImageUpload,
    resolveResizedImageProportion,
  } from '../image';
  import type { ExternalElement, ExternalElementData } from '@typie/editor-ffi/browser';

  type ImageData = Extract<ExternalElementData, { type: 'image' }>;

  type Props = {
    element: ExternalElement;
  };

  let { element }: Props = $props();

  const ctx = getEditorContext();
  const editor = $derived(ctx.editor);

  let isResizing = $state(false);
  let liveProportion = $state<number | null>(null);
  let initialResizeData: { x: number; width: number; reverse: boolean } | null = null;
  let enlarged = $state(false);
  let containerEl = $state<HTMLDivElement>();

  const imageData = $derived(element.data as ImageData);

  const imageQuery = createQuery(
    graphql(`
      query FfiExternalImage_Query($imageId: ID!) {
        image(imageId: $imageId) {
          id
          url
          originalUrl
          width
          height
          ratio
          placeholder
        }
      }
    `),
    () => ({ imageId: imageData.id ?? '' }),
    () => ({ skip: !imageData.id }),
  );

  const asset = $derived(imageData.id ? (editor?.imageAssets.get(imageData.id) ?? imageQuery.data?.image ?? undefined) : undefined);
  const inflight = $derived(editor?.inflightImages.get(element.node_id));

  const activeProportion = $derived(liveProportion ?? imageData.proportion ?? 100);

  const presentation = $derived(
    computeImagePresentation({
      proportion: activeProportion,
      boundsWidth: element.bounds.width,
      imageId: imageData.id,
      asset: asset ?? undefined,
      inflight: inflight ?? undefined,
    }),
  );

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 4,
    middleware: [flip(), hide()],
  });

  $effect(() => {
    if (!presentation.hasImage) {
      enlarged = false;
    }
  });

  const dispatchProportion = (nextProportion: number) => {
    if (!editor) return;
    editor.enqueue(
      createSetImageAttrsMessage({
        nodeId: element.node_id,
        currentId: imageData.id,
        currentProportion: imageData.proportion || 100,
        nextProportion,
      }),
    );
  };

  const handleDelete = () => {
    if (!editor) return;
    editor.pendingImageFilesByNode.delete(element.node_id);
    editor.inflightImages.delete(element.node_id);
    editor.enqueue(createDeleteImageMessage(element.node_id));
    editor.focus();
  };

  const handleResizeStart = (event: PointerEvent, reverse: boolean) => {
    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture(event.pointerId);
    event.stopPropagation();
    event.preventDefault();

    isResizing = true;
    initialResizeData = {
      x: event.clientX,
      width: presentation.width,
      reverse,
    };
  };

  const handleResize = (event: PointerEvent) => {
    const target = event.currentTarget as HTMLElement;
    if (!target.hasPointerCapture(event.pointerId) || !initialResizeData) {
      return;
    }

    const boundsWidth = element.bounds.width;
    if (boundsWidth <= 0) return;

    const resized = resolveResizedImageProportion({
      boundsWidth,
      originalWidth: presentation.originalWidth,
      initialWidth: initialResizeData.width,
      initialClientX: initialResizeData.x,
      nextClientX: event.clientX,
      reverse: initialResizeData.reverse,
    });
    liveProportion = resized.proportion;
  };

  const handleResizeEnd = (event: PointerEvent) => {
    const target = event.currentTarget as HTMLElement;
    target.releasePointerCapture(event.pointerId);

    const finalProportion = liveProportion;
    isResizing = false;
    liveProportion = null;

    if (finalProportion != null) {
      dispatchProportion(finalProportion);
    }
    editor?.focus();
  };

  $effect(() => {
    if (!isResizing) {
      liveProportion = null;
    }
  });

  const getImageDimensions = (src: string): Promise<{ width: number; height: number }> =>
    new Promise((resolve, reject) => {
      const img = new Image();
      img.addEventListener('load', () => resolve({ width: img.naturalWidth, height: img.naturalHeight }));
      img.addEventListener('error', () => reject(new Error('Failed to load image')));
      img.src = src;
    });

  const runUpload = async (file: File) => {
    const currentEditor = editor;
    if (!currentEditor) {
      return;
    }

    await processImageUpload({
      file,
      nodeId: element.node_id,
      currentId: imageData.id,
      currentProportion: imageData.proportion || 100,
      editor: currentEditor,
      getImageDimensions,
      uploadImage: uploadBlobAsImage,
      createObjectUrl: URL.createObjectURL,
      revokeObjectUrl: URL.revokeObjectURL,
      onFailure: (err) => {
        console.error('Image upload failed:', err);
        Toast.error(`${file.name} 이미지 업로드에 실패했습니다.`);
      },
    });
  };

  $effect(() => {
    const currentEditor = editor;
    if (!currentEditor || imageData.id) {
      return;
    }

    const file = currentEditor.dequeuePendingImageFile(element.node_id);
    if (file) {
      void runUpload(file);
    }
  });

  const handlePickImage = () => {
    const picker = document.createElement('input');
    picker.type = 'file';
    picker.accept = 'image/*';

    picker.addEventListener('change', () => {
      const file = picker.files?.[0];
      if (!file) return;
      void runUpload(file);
    });

    picker.click();
  };
</script>

{#if presentation.hasImage}
  <div
    bind:this={containerEl}
    style:width={`${presentation.width}px`}
    style:height={`${presentation.height}px`}
    class={css({ position: 'relative', margin: '[0 auto]' })}
  >
    <Img
      style={css.raw({ width: 'full', borderRadius: '4px' })}
      alt="본문 이미지"
      placeholder={presentation.placeholder ?? undefined}
      progressive
      ratio={presentation.originalHeight > 0 ? presentation.originalWidth / presentation.originalHeight : undefined}
      size="full"
      url={presentation.url ?? ''}
    />

    {#if presentation.isUploading}
      <div class={center({ position: 'absolute', inset: '0', backgroundColor: 'white/50' })}>
        <RingSpinner style={css.raw({ size: '24px', color: 'text.disabled' })} />
      </div>
    {/if}

    {#if element.is_selected}
      <div class={css({ position: 'absolute', top: '10px', right: '10px', display: 'flex', gap: '8px', zIndex: '10' })}>
        <button
          class={css({
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: '4px',
            size: '28px',
            color: 'text.bright',
            backgroundColor: '[#363839/70]',
          })}
          aria-label="이미지 확대"
          onclick={() => (enlarged = true)}
          onpointerdown={(event) => event.stopPropagation()}
          type="button"
        >
          <Icon icon={ZoomInIcon} size={16} />
        </button>

        <button
          class={css({
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: '4px',
            size: '28px',
            color: 'text.bright',
            backgroundColor: '[#363839/70]',
          })}
          aria-label="이미지 삭제"
          onclick={handleDelete}
          onpointerdown={(event) => event.stopPropagation()}
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
            zIndex: '10',
            pointerEvents: 'auto',
          })}
          aria-label="이미지 크기 조절"
          onpointerdown={(event) => handleResizeStart(event, true)}
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
            zIndex: '10',
            pointerEvents: 'auto',
          })}
          aria-label="이미지 크기 조절"
          onpointerdown={(event) => handleResizeStart(event, false)}
          onpointermove={handleResize}
          onpointerup={handleResizeEnd}
          type="button"
        ></button>
      </div>
    {/if}
  </div>
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
    <div
      class={flex({
        align: 'center',
        gap: '12px',
        paddingX: '14px',
        paddingY: '12px',
        fontSize: '14px',
        color: 'text.disabled',
      })}
    >
      <Icon icon={ImageIcon} size={20} />
      {presentation.isResolvingAsset ? '이미지를 불러오는 중...' : '이미지'}
    </div>

    {#if presentation.isResolvingAsset}
      <div class={css({ marginRight: '14px' })}>
        <RingSpinner style={css.raw({ size: '16px', color: 'text.disabled' })} />
      </div>
    {:else}
      <div onpointerdown={(event) => event.stopPropagation()} role="none">
        <Menu>
          {#snippet button({ open }: { open: boolean })}
            <div
              class={css(
                {
                  marginRight: '12px',
                  borderRadius: '4px',
                  padding: '2px',
                  color: 'text.disabled',
                  _hover: { backgroundColor: 'interactive.hover' },
                },
                open && { backgroundColor: 'interactive.hover' },
              )}
            >
              <Icon icon={EllipsisIcon} size={20} />
            </div>
          {/snippet}

          <MenuItem onclick={handleDelete} variant="danger">
            <Icon icon={Trash2Icon} size={12} />
            <span>삭제</span>
          </MenuItem>
        </Menu>
      </div>
    {/if}
  </div>
{/if}

{#if element.is_selected && !presentation.hasImage && !presentation.isResolvingAsset}
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
    onclick={handlePickImage}
    onpointerdown={(event) => event.stopPropagation()}
    type="button"
    use:floating
  >
    <Icon icon={ImageIcon} size={14} />
    이미지 선택
  </button>
{/if}

{#if enlarged && presentation.hasImage && presentation.url && containerEl}
  <ExternalImageEnlarge
    onclose={() => (enlarged = false)}
    placeholder={presentation.placeholder ?? undefined}
    ratio={presentation.originalHeight > 0 ? presentation.originalWidth / presentation.originalHeight : undefined}
    referenceEl={containerEl}
    url={presentation.url}
  />
{/if}
