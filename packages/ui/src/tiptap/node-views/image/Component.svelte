<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tick } from 'svelte';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import ImageIcon from '~icons/lucide/image';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { createFloatingActions } from '../../../actions';
  import { Button, Icon, Img, Menu, MenuItem, RingSpinner } from '../../../components';
  import { Toast } from '../../../notification';
  import { clamp } from '../../../utils/number';
  import { mmToPx } from '../../../utils/unit';
  import { getEditorContext, getNodeView, NodeView } from '../../lib';
  import Enlarge from './Enlarge.svelte';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, editor, selected, updateAttributes, deleteNode, getPos, HTMLAttributes }: Props = $props();

  let pendingFiles = $state<File[]>([]);
  let inflightUrl = $state<string>();

  let attrs = $state(node.attrs);
  $effect(() => {
    attrs = node.attrs;
  });

  const pageLayout = $derived(editor?.current.storage.page?.layout);
  const context = getEditorContext();

  const maxContentHeight = $derived(pageLayout ? mmToPx(pageLayout.height - pageLayout.marginTop - pageLayout.marginBottom) : undefined);

  $effect(() => {
    if (pendingFiles.length > 0) {
      processPendingFiles();
    }
  });

  $effect(() => {
    if (maxContentHeight && (attrs.id || inflightUrl)) {
      checkAndAdjustProportion();
    }
  });

  const processPendingFiles = async () => {
    if (pendingFiles.length === 0) return;

    const [firstFile, ...restFiles] = pendingFiles;
    pendingFiles = [];

    const objectUrl = URL.createObjectURL(firstFile);
    inflightUrl = objectUrl;

    try {
      if (restFiles.length > 0 && editor?.current) {
        const currentPos = getPos();
        if (currentPos !== undefined) {
          const insertPos = currentPos + node.nodeSize;
          editor.current
            .chain()
            .insertContentAt(insertPos, {
              type: 'image',
            })
            .focus()
            .run();

          await tick();
          const nextNodeView = getNodeView(editor.current.view, insertPos);
          if (nextNodeView?.handle) {
            nextNodeView.handle(new CustomEvent('pending-files', { detail: { files: restFiles } }));
          }
        }
      }

      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const uploadedAttrs = await editor!.current.storage.uploadBlobAsImage(firstFile);
      inflightUrl = undefined;
      updateAttributes(uploadedAttrs);
    } catch {
      inflightUrl = undefined;
      Toast.error(`${firstFile.name} 이미지 업로드에 실패했습니다.`);
    } finally {
      URL.revokeObjectURL(objectUrl);
    }
  };

  let pickerOpened = $state(false);
  $effect(() => {
    pickerOpened = selected;
  });

  let enlarged = $state(false);

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 4,
    onClickOutside: () => {
      pickerOpened = false;
    },
  });

  const handleUpload = async () => {
    const picker = document.createElement('input');
    picker.type = 'file';
    picker.accept = 'image/*';
    picker.multiple = true;

    picker.addEventListener('change', async () => {
      pickerOpened = false;

      const files = picker.files;
      if (!files || files.length === 0) {
        return;
      }

      pendingFiles = [...files];
    });

    picker.click();
  };

  export const handle = (event: CustomEvent) => {
    if (event.type === 'inflight') {
      inflightUrl = event.detail.url;
    } else if (event.type === 'success') {
      inflightUrl = undefined;
      updateAttributes(event.detail.attrs);
    } else if (event.type === 'error') {
      inflightUrl = undefined;
    } else if (event.type === 'pending-files') {
      pendingFiles = event.detail.files;
    }
  };

  let containerEl = $state<HTMLDivElement>();
  let proportion = $state(node.attrs.proportion);

  const calculateConstrainedProportion = (proposedProportion: number): { proportion: number; minProportion: number } => {
    if (!maxContentHeight || !containerEl) {
      return { proportion: proposedProportion, minProportion: 0.1 };
    }

    const imgElement = containerEl.querySelector('img') as HTMLImageElement;
    if (!imgElement) {
      return { proportion: proposedProportion, minProportion: 0.1 };
    }

    const parentWidth = containerEl.parentElement?.clientWidth || 0;
    const proposedWidth = parentWidth * proposedProportion;

    const currentRect = imgElement.getBoundingClientRect();
    const aspectRatio = currentRect.width / currentRect.height;
    const proposedHeight = proposedWidth / aspectRatio;

    // NOTE: 이미지가 maxContentHeight를 넘지 않기 위한 최대 너비
    const maxWidthForHeight = maxContentHeight * aspectRatio;
    const minProportionForHeight = maxWidthForHeight / parentWidth;

    // NOTE: 더 작은 값을 최소 proportion으로 사용
    const minProportion = Math.min(0.1, minProportionForHeight);

    let constrainedProportion = proposedProportion;
    if (proposedHeight > maxContentHeight) {
      constrainedProportion = maxWidthForHeight / parentWidth;
    }

    return { proportion: constrainedProportion, minProportion };
  };

  const checkAndAdjustProportion = () => {
    const { proportion: constrainedProportion, minProportion } = calculateConstrainedProportion(proportion);
    const clampedProportion = clamp(constrainedProportion, minProportion, 1);
    if (clampedProportion !== proportion) {
      proportion = clampedProportion;
      updateAttributes({ proportion });
    }
  };

  let initialResizeData: {
    x: number;
    width: number;
    proportion: number;
    reverse: boolean;
  } | null = null;

  const handleResizeStart = (event: PointerEvent, reverse: boolean) => {
    if (!containerEl) {
      return;
    }

    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture(event.pointerId);

    initialResizeData = {
      x: event.clientX,
      width: containerEl.clientWidth,
      proportion,
      reverse,
    };
  };

  const handleResize = (event: PointerEvent) => {
    const target = event.currentTarget as HTMLElement;
    if (!target.hasPointerCapture(event.pointerId) || !initialResizeData || !containerEl) {
      return;
    }

    const dx = (event.clientX - initialResizeData.x) * (initialResizeData.reverse ? -1 : 1);
    const proposedProportion = ((initialResizeData.width + dx * 2) / initialResizeData.width) * initialResizeData.proportion;

    const { proportion: constrainedProportion, minProportion } = calculateConstrainedProportion(proposedProportion);
    proportion = clamp(constrainedProportion, minProportion, 1);
  };

  const handleResizeEnd = (event: PointerEvent) => {
    const target = event.currentTarget as HTMLElement;
    target.releasePointerCapture(event.pointerId);
    updateAttributes({ proportion });
  };

  $effect(() => {
    return () => {
      if (inflightUrl && inflightUrl.startsWith('blob:')) {
        URL.revokeObjectURL(inflightUrl);
      }
    };
  });
</script>

<NodeView style={css.raw({ display: 'flex', justifyContent: 'center', width: 'full' })} {...HTMLAttributes}>
  <div
    bind:this={containerEl}
    style:width={`${proportion * 100}%`}
    class={cx('group', css({ position: 'relative' }))}
    data-drag-handle
    draggable
  >
    {#if attrs.id || inflightUrl}
      {#if attrs.id}
        <div style:max-height={maxContentHeight ? `${maxContentHeight}px` : undefined}>
          <Img
            style={css.raw({ width: 'full', borderRadius: '4px' }, !editor?.current.isEditable && { cursor: 'zoom-in' })}
            alt="본문 이미지"
            onclick={() => !editor?.current.isEditable && (enlarged = true)}
            onload={checkAndAdjustProportion}
            placeholder={attrs.placeholder}
            progressive={!context?.pdf}
            ratio={attrs.ratio}
            role="button"
            size="full"
            url={attrs.url}
          />
        </div>
      {:else if inflightUrl}
        <img
          style:max-height={maxContentHeight ? `${maxContentHeight}px` : undefined}
          class={css({ width: 'full', borderRadius: '4px' })}
          alt="본문 이미지"
          onerror={(e) => {
            (e.currentTarget as HTMLImageElement).src = 'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7';
          }}
          onload={checkAndAdjustProportion}
          src={inflightUrl}
        />
        <div class={center({ position: 'absolute', inset: '0', backgroundColor: 'white/50', zIndex: 'editor' })}>
          <RingSpinner style={css.raw({ size: '24px', color: 'text.disabled' })} />
        </div>
      {/if}

      {#if editor?.current.isEditable && !window.__webview__}
        <button
          class={css({
            position: 'absolute',
            top: '10px',
            right: '10px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: '4px',
            size: '28px',
            color: 'text.bright',
            backgroundColor: '[#363839/70]',
            opacity: '0',
            transition: 'opacity',
            zIndex: 'editor',
            _hover: { backgroundColor: '[#363839/40]' },
            _groupHover: { opacity: '100' },
          })}
          onclick={() => deleteNode()}
          type="button"
        >
          <Icon icon={Trash2Icon} size={16} />
        </button>

        <div
          class={flex({
            position: 'absolute',
            top: '0',
            bottom: '0',
            left: '10px',
            alignItems: 'center',
            pointerEvents: 'none',
          })}
        >
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

        <div
          class={flex({
            position: 'absolute',
            top: '0',
            bottom: '0',
            right: '10px',
            alignItems: 'center',
            pointerEvents: 'none',
          })}
        >
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
              pointerEvents: 'auto',
              _hover: { backgroundColor: 'white/40' },
              _groupHover: { opacity: '100' },
            })}
            aria-label="이미지 크기 조절"
            onpointerdown={(event) => {
              event.preventDefault();
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
          이미지
        </div>

        {#if editor?.current.isEditable && !window.__webview__}
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

            <MenuItem onclick={() => deleteNode()} variant="danger">
              <Icon icon={Trash2Icon} size={12} />
              <span>삭제</span>
            </MenuItem>
          </Menu>
        {/if}
      </div>
    {/if}
  </div>
</NodeView>

{#if pickerOpened && !attrs.id && !inflightUrl && editor?.current.isEditable && !window.__webview__}
  <div
    class={center({
      flexDirection: 'column',
      gap: '12px',
      borderWidth: '1px',
      borderRadius: '12px',
      padding: '12px',
      width: '380px',
      backgroundColor: 'surface.default',
      boxShadow: 'small',
      zIndex: 'editor',
    })}
    use:floating
  >
    <span class={css({ fontSize: '13px', color: 'text.muted' })}>아래 버튼을 클릭해 이미지를 선택하세요</span>

    <Button style={css.raw({ width: 'full' })} onclick={handleUpload} size="sm" variant="secondary">이미지 선택</Button>
  </div>
{/if}

{#if enlarged}
  <Enlarge {node} onclose={() => (enlarged = false)} referenceEl={containerEl} />
{/if}
