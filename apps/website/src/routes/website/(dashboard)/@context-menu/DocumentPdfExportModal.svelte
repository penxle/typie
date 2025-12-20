<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Button, Checkbox, HorizontalDivider, Icon, Modal, Select, TextInput } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { clamp, createDefaultPageLayout, getMaxMargin, PAGE_LAYOUT_OPTIONS, PAGE_SIZE_MAP } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import FileIcon from '~icons/lucide/file';
  import MoveHorizontalIcon from '~icons/lucide/move-horizontal';
  import MoveVerticalIcon from '~icons/lucide/move-vertical';
  import PanelBottomDashedIcon from '~icons/lucide/panel-bottom-dashed';
  import PanelLeftDashedIcon from '~icons/lucide/panel-left-dashed';
  import PanelRightDashedIcon from '~icons/lucide/panel-right-dashed';
  import PanelTopDashedIcon from '~icons/lucide/panel-top-dashed';
  import RulerDimensionLineIcon from '~icons/lucide/ruler-dimension-line';
  import { graphql } from '$graphql';
  import type { LayoutMode } from '@typie/editor';
  import type { PageLayout, PageLayoutPreset } from '@typie/ui/utils';

  const PX_TO_MM = 25.4 / 96;

  type Props = {
    open: boolean;
    documentId: string;
    slug: string;
    via: 'tree' | 'editor';
    onClose: () => void;
  };

  let { open = $bindable(), documentId, slug, via, onClose }: Props = $props();

  const app = getAppContext();

  let loaded = $state(false);
  let isExporting = $state(false);
  let useCurrentSettings = $state(false);
  let pageLayout = $state<PageLayout>(app.preference.current.lastPdfPageLayout ?? createDefaultPageLayout('a4'));

  const documentQuery = graphql(`
    query DocumentPdfExportModal_Document_Query($slug: String!) @client {
      document(slug: $slug) {
        id
        layoutMode
      }
    }
  `);

  const exportDocumentAsPdf = graphql(`
    mutation DocumentPdfExportModal_ExportDocumentAsPdf_Mutation($input: ExportDocumentAsPdfInput!) {
      exportDocumentAsPdf(input: $input) {
        data
        filename
      }
    }
  `);

  const convertLayoutModeToPageLayout = (mode: LayoutMode): PageLayout | undefined => {
    if (mode.type !== 'paginated') return undefined;
    return {
      width: Math.round(mode.pageWidth * PX_TO_MM),
      height: Math.round(mode.pageHeight * PX_TO_MM),
      marginTop: Math.round(mode.pageMarginTop * PX_TO_MM),
      marginBottom: Math.round(mode.pageMarginBottom * PX_TO_MM),
      marginLeft: Math.round(mode.pageMarginLeft * PX_TO_MM),
      marginRight: Math.round(mode.pageMarginRight * PX_TO_MM),
    };
  };

  const load = async () => {
    loaded = false;
    await documentQuery.load({ slug });
    loaded = true;
  };

  $effect(() => {
    if (open) {
      load();
    }
  });

  $effect(() => {
    if (loaded && $documentQuery) {
      const layoutMode = $documentQuery.document.layoutMode as LayoutMode;
      const isPaginated = layoutMode.type === 'paginated';
      const converted = convertLayoutModeToPageLayout(layoutMode);

      useCurrentSettings = isPaginated && !!converted;
      if (converted && isPaginated) {
        pageLayout = { ...converted };
      } else {
        pageLayout = app.preference.current.lastPdfPageLayout
          ? { ...app.preference.current.lastPdfPageLayout }
          : createDefaultPageLayout('a4');
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

  const handleConfirm = async (currentPageLayout: PageLayout | undefined) => {
    const layout = useCurrentSettings && currentPageLayout ? currentPageLayout : pageLayout;

    if (!useCurrentSettings) {
      app.preference.current.lastPdfPageLayout = pageLayout;
    }

    isExporting = true;
    const result = await exportDocumentAsPdf({
      documentId,
      width: layout.width,
      height: layout.height,
      marginTop: layout.marginTop,
      marginBottom: layout.marginBottom,
      marginLeft: layout.marginLeft,
      marginRight: layout.marginRight,
    });
    isExporting = false;

    downloadPdf(result.data, result.filename);
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
  {#if loaded && $documentQuery}
    {@const layoutMode = $documentQuery.document.layoutMode as LayoutMode}
    {@const currentPageEnabled = layoutMode.type === 'paginated'}
    {@const currentPageLayout = convertLayoutModeToPageLayout(layoutMode)}

    <div class={css({ padding: '20px' })}>
      <h2 class={css({ fontSize: '18px', fontWeight: 'semibold', marginBottom: '16px' })}>PDF로 내보내기</h2>

      <div class={flex({ flexDirection: 'column', gap: '16px', paddingY: '8px' })}>
        {#if currentPageEnabled && currentPageLayout}
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
                items={PAGE_LAYOUT_OPTIONS}
                onselect={(value: PageLayoutPreset | 'custom') => {
                  if (value === 'custom') return;
                  pageLayout = createDefaultPageLayout(value);
                }}
                value={(Object.entries(PAGE_SIZE_MAP).find(
                  ([, dimension]) => dimension.width === pageLayout.width && dimension.height === pageLayout.height,
                )?.[0] as PageLayoutPreset) ?? ('custom' as const)}
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
                    pageLayout = { ...pageLayout, width: value };
                  }}
                  size="sm"
                  type="number"
                  value={pageLayout.width}
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
                    pageLayout = { ...pageLayout, height: value };
                  }}
                  size="sm"
                  type="number"
                  value={pageLayout.height}
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
                  max={String(getMaxMargin('top', pageLayout))}
                  min="0"
                  oninput={(e) => {
                    const target = e.target as HTMLInputElement;
                    const value = clamp(Number(target.value), 0, getMaxMargin('top', pageLayout));
                    target.value = String(value);
                    pageLayout = { ...pageLayout, marginTop: value };
                  }}
                  size="sm"
                  type="number"
                  value={pageLayout.marginTop}
                />
              </div>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Icon style={css.raw({ size: '14px', color: 'text.subtle' })} icon={PanelBottomDashedIcon} />
                <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>아래</div>
                <TextInput
                  style={css.raw({ width: '100px' })}
                  disabled={useCurrentSettings}
                  max={String(getMaxMargin('bottom', pageLayout))}
                  min="0"
                  oninput={(e) => {
                    const target = e.target as HTMLInputElement;
                    const value = clamp(Number(target.value), 0, getMaxMargin('bottom', pageLayout));
                    target.value = String(value);
                    pageLayout = { ...pageLayout, marginBottom: value };
                  }}
                  size="sm"
                  type="number"
                  value={pageLayout.marginBottom}
                />
              </div>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Icon style={css.raw({ size: '14px', color: 'text.subtle' })} icon={PanelLeftDashedIcon} />
                <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>왼쪽</div>
                <TextInput
                  style={css.raw({ width: '100px' })}
                  disabled={useCurrentSettings}
                  max={String(getMaxMargin('left', pageLayout))}
                  min="0"
                  oninput={(e) => {
                    const target = e.target as HTMLInputElement;
                    const value = clamp(Number(target.value), 0, getMaxMargin('left', pageLayout));
                    target.value = String(value);
                    pageLayout = { ...pageLayout, marginLeft: value };
                  }}
                  size="sm"
                  type="number"
                  value={pageLayout.marginLeft}
                />
              </div>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Icon style={css.raw({ size: '14px', color: 'text.subtle' })} icon={PanelRightDashedIcon} />
                <div class={css({ fontSize: '12px', color: 'text.subtle', width: '32px' })}>오른쪽</div>
                <TextInput
                  style={css.raw({ width: '100px' })}
                  disabled={useCurrentSettings}
                  max={String(getMaxMargin('right', pageLayout))}
                  min="0"
                  oninput={(e) => {
                    const target = e.target as HTMLInputElement;
                    const value = clamp(Number(target.value), 0, getMaxMargin('right', pageLayout));
                    target.value = String(value);
                    pageLayout = { ...pageLayout, marginRight: value };
                  }}
                  size="sm"
                  type="number"
                  value={pageLayout.marginRight}
                />
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class={flex({ gap: '8px', justifyContent: 'flex-end', marginTop: '20px' })}>
        <Button onclick={onClose} variant="secondary">취소</Button>
        <Button loading={isExporting} onclick={() => handleConfirm(currentPageLayout)}>내보내기</Button>
      </div>
    </div>
  {/if}
</Modal>
