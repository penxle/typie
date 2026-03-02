<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Button, Checkbox, HorizontalDivider, Icon, Modal, Select, TextInput } from '@typie/ui/components';
  import { clamp } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import FileIcon from '~icons/lucide/file';
  import MoveHorizontalIcon from '~icons/lucide/move-horizontal';
  import MoveVerticalIcon from '~icons/lucide/move-vertical';
  import PanelBottomDashedIcon from '~icons/lucide/panel-bottom-dashed';
  import PanelLeftDashedIcon from '~icons/lucide/panel-left-dashed';
  import PanelRightDashedIcon from '~icons/lucide/panel-right-dashed';
  import PanelTopDashedIcon from '~icons/lucide/panel-top-dashed';
  import RulerDimensionLineIcon from '~icons/lucide/ruler-dimension-line';
  import { createPaginatedLayout, getMaxMargin, mmToPx, pxToMm } from '$lib/editor/utils';
  import { values } from '$lib/editor/values';
  import { graphql } from '$mearie';
  import type { LayoutMode } from '@typie/editor';
  import type { PageLayout, PageLayoutPreset } from '$lib/editor/utils';

  type Props = {
    open: boolean;
    documentId: string;
    slug: string;
    via: 'tree' | 'editor';
    onClose: () => void;
  };

  let { open = $bindable(), documentId, slug, via, onClose }: Props = $props();

  let loaded = $state(false);
  let isExporting = $state(false);
  let useCurrentSettings = $state(false);
  let pageLayout = $state<PageLayout>(createPaginatedLayout('a4'));

  const documentQuery = createQuery(
    graphql(`
      query DocumentPdfExportModal_Document_Query($slug: String!) {
        document(slug: $slug) {
          id
          layoutMode
        }
      }
    `),
    () => ({ slug }),
    () => ({ skip: !open }),
  );

  const [exportDocumentAsPdf] = createMutation(
    graphql(`
      mutation DocumentPdfExportModal_ExportDocumentAsPdf_Mutation($input: ExportDocumentAsPdfInput!) {
        exportDocumentAsPdf(input: $input) {
          data
          filename
        }
      }
    `),
  );

  $effect(() => {
    if (open) {
      loaded = false;
      documentQuery.refetch();
    }
  });

  $effect(() => {
    if (open && documentQuery.data && !documentQuery.loading) {
      loaded = true;
    }
  });

  $effect(() => {
    if (loaded && documentQuery.data) {
      const layoutMode = documentQuery.data.document.layoutMode as LayoutMode;
      const isPaginated = layoutMode.type === 'paginated';

      useCurrentSettings = isPaginated;
      if (isPaginated) {
        pageLayout = {
          pageWidth: layoutMode.pageWidth,
          pageHeight: layoutMode.pageHeight,
          pageMarginTop: layoutMode.pageMarginTop,
          pageMarginBottom: layoutMode.pageMarginBottom,
          pageMarginLeft: layoutMode.pageMarginLeft,
          pageMarginRight: layoutMode.pageMarginRight,
        };
      } else {
        pageLayout = createPaginatedLayout('a4');
      }
    }
  });

  const downloadPdf = (data: string, filename: string) => {
    const blob = new Blob([Uint8Array.fromBase64(data)], { type: 'application/pdf' });
    const url = URL.createObjectURL(blob);

    const a = window.document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();

    URL.revokeObjectURL(url);
  };

  const handleConfirm = async () => {
    isExporting = true;
    const result = await exportDocumentAsPdf({
      input: {
        documentId,
        pageWidth: Math.round(pageLayout.pageWidth),
        pageHeight: Math.round(pageLayout.pageHeight),
        pageMarginTop: Math.round(pageLayout.pageMarginTop),
        pageMarginBottom: Math.round(pageLayout.pageMarginBottom),
        pageMarginLeft: Math.round(pageLayout.pageMarginLeft),
        pageMarginRight: Math.round(pageLayout.pageMarginRight),
      },
    });
    isExporting = false;

    downloadPdf(result.exportDocumentAsPdf.data, result.exportDocumentAsPdf.filename);
    mixpanel.track('export_document_pdf', { via });
    onClose();
  };
</script>

<Modal
  style={css.raw({
    maxWidth: '400px',
  })}
  loading={!loaded}
  onclose={onClose}
  bind:open
>
  {#if loaded && documentQuery.data}
    {@const layoutMode = documentQuery.data.document.layoutMode as LayoutMode}
    {@const currentPageEnabled = layoutMode.type === 'paginated'}

    <div class={css({ padding: '20px' })}>
      <h2 class={css({ fontSize: '18px', fontWeight: 'semibold', marginBottom: '16px' })}>PDF로 내보내기</h2>

      <div class={flex({ flexDirection: 'column', gap: '16px', paddingY: '8px' })}>
        {#if currentPageEnabled}
          <Checkbox bind:checked={useCurrentSettings}>
            <span class={css({ fontSize: '14px' })}>현재 페이지 설정 사용</span>
          </Checkbox>
          <HorizontalDivider />
        {/if}

        <div
          class={flex({
            flexDirection: 'column',
            gap: '16px',
            opacity: useCurrentSettings ? '50' : '100',
            pointerEvents: useCurrentSettings ? 'none' : 'auto',
          })}
        >
          <div class={flex({ flexDirection: 'column', gap: '8px' })}>
            <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px' })}>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Icon style={css.raw({ color: 'text.faint' })} icon={FileIcon} />
                <div class={css({ fontSize: '13px', color: 'text.subtle' })}>페이지 크기 (mm)</div>
              </div>
              <Select
                disabled={useCurrentSettings}
                items={[...values.pageLayout, { label: '직접 지정', value: 'custom' }]}
                onselect={(value: string) => {
                  if (value === 'custom') return;
                  pageLayout = createPaginatedLayout(value as PageLayoutPreset);
                }}
                value={values.pageLayout.find(
                  (p) => p.layout.pageWidth === pageLayout.pageWidth && p.layout.pageHeight === pageLayout.pageHeight,
                )?.value ?? 'custom'}
              />
            </div>

            <div class={grid({ columns: 2, columnGap: '12px', rowGap: '8px', paddingLeft: '8px' })}>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Icon style={css.raw({ size: '14px', color: 'text.subtle' })} icon={MoveHorizontalIcon} />
                <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>너비</div>
                <TextInput
                  style={css.raw({ width: '100px' })}
                  disabled={useCurrentSettings}
                  min="100"
                  onchange={(e) => {
                    const target = e.target as HTMLInputElement;
                    const value = Math.max(100, Number(target.value));
                    target.value = String(value);
                    pageLayout = { ...pageLayout, pageWidth: mmToPx(value) };
                  }}
                  size="sm"
                  type="number"
                  value={pxToMm(pageLayout.pageWidth)}
                />
              </div>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Icon style={css.raw({ size: '14px', color: 'text.subtle' })} icon={MoveVerticalIcon} />
                <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>높이</div>
                <TextInput
                  style={css.raw({ width: '100px' })}
                  disabled={useCurrentSettings}
                  min="100"
                  onchange={(e) => {
                    const target = e.target as HTMLInputElement;
                    const value = Math.max(100, Number(target.value));
                    target.value = String(value);
                    pageLayout = { ...pageLayout, pageHeight: mmToPx(value) };
                  }}
                  size="sm"
                  type="number"
                  value={pxToMm(pageLayout.pageHeight)}
                />
              </div>
            </div>
          </div>

          <div class={flex({ flexDirection: 'column', gap: '8px' })}>
            <div class={flex({ alignItems: 'center', gap: '8px' })}>
              <Icon style={css.raw({ color: 'text.faint' })} icon={RulerDimensionLineIcon} />
              <div class={css({ fontSize: '13px', color: 'text.subtle' })}>여백 (mm)</div>
            </div>
            <div class={grid({ columns: 2, columnGap: '12px', rowGap: '8px', paddingLeft: '8px' })}>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Icon style={css.raw({ size: '14px', color: 'text.subtle' })} icon={PanelTopDashedIcon} />
                <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>위</div>
                <TextInput
                  style={css.raw({ width: '100px' })}
                  disabled={useCurrentSettings}
                  max={String(pxToMm(getMaxMargin('top', pageLayout)))}
                  min="0"
                  oninput={(e) => {
                    const target = e.target as HTMLInputElement;
                    const valuePx = clamp(mmToPx(Number(target.value)), 0, getMaxMargin('top', pageLayout));
                    target.value = String(pxToMm(valuePx));
                    pageLayout = { ...pageLayout, pageMarginTop: valuePx };
                  }}
                  size="sm"
                  type="number"
                  value={pxToMm(pageLayout.pageMarginTop)}
                />
              </div>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Icon style={css.raw({ size: '14px', color: 'text.subtle' })} icon={PanelBottomDashedIcon} />
                <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>아래</div>
                <TextInput
                  style={css.raw({ width: '100px' })}
                  disabled={useCurrentSettings}
                  max={String(pxToMm(getMaxMargin('bottom', pageLayout)))}
                  min="0"
                  oninput={(e) => {
                    const target = e.target as HTMLInputElement;
                    const valuePx = clamp(mmToPx(Number(target.value)), 0, getMaxMargin('bottom', pageLayout));
                    target.value = String(pxToMm(valuePx));
                    pageLayout = { ...pageLayout, pageMarginBottom: valuePx };
                  }}
                  size="sm"
                  type="number"
                  value={pxToMm(pageLayout.pageMarginBottom)}
                />
              </div>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Icon style={css.raw({ size: '14px', color: 'text.subtle' })} icon={PanelLeftDashedIcon} />
                <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>왼쪽</div>
                <TextInput
                  style={css.raw({ width: '100px' })}
                  disabled={useCurrentSettings}
                  max={String(pxToMm(getMaxMargin('left', pageLayout)))}
                  min="0"
                  oninput={(e) => {
                    const target = e.target as HTMLInputElement;
                    const valuePx = clamp(mmToPx(Number(target.value)), 0, getMaxMargin('left', pageLayout));
                    target.value = String(pxToMm(valuePx));
                    pageLayout = { ...pageLayout, pageMarginLeft: valuePx };
                  }}
                  size="sm"
                  type="number"
                  value={pxToMm(pageLayout.pageMarginLeft)}
                />
              </div>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Icon style={css.raw({ size: '14px', color: 'text.subtle' })} icon={PanelRightDashedIcon} />
                <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>오른쪽</div>
                <TextInput
                  style={css.raw({ width: '100px' })}
                  disabled={useCurrentSettings}
                  max={String(pxToMm(getMaxMargin('right', pageLayout)))}
                  min="0"
                  oninput={(e) => {
                    const target = e.target as HTMLInputElement;
                    const valuePx = clamp(mmToPx(Number(target.value)), 0, getMaxMargin('right', pageLayout));
                    target.value = String(pxToMm(valuePx));
                    pageLayout = { ...pageLayout, pageMarginRight: valuePx };
                  }}
                  size="sm"
                  type="number"
                  value={pxToMm(pageLayout.pageMarginRight)}
                />
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class={flex({ gap: '8px', justifyContent: 'flex-end', marginTop: '20px' })}>
        <Button onclick={onClose} variant="secondary">취소</Button>
        <Button loading={isExporting} onclick={handleConfirm}>내보내기</Button>
      </div>
    </div>
  {/if}
</Modal>
