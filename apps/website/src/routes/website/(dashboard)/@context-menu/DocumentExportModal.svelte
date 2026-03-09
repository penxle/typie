<script lang="ts">
  import { createFragment, createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Button, Checkbox, FullAccessBadge, HorizontalDivider, Icon, Modal, Select, TextInput } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { clamp } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import FileIcon from '~icons/lucide/file';
  import InfoIcon from '~icons/lucide/info';
  import MoveHorizontalIcon from '~icons/lucide/move-horizontal';
  import MoveVerticalIcon from '~icons/lucide/move-vertical';
  import PanelBottomDashedIcon from '~icons/lucide/panel-bottom-dashed';
  import PanelLeftDashedIcon from '~icons/lucide/panel-left-dashed';
  import PanelRightDashedIcon from '~icons/lucide/panel-right-dashed';
  import PanelTopDashedIcon from '~icons/lucide/panel-top-dashed';
  import RulerDimensionLineIcon from '~icons/lucide/ruler-dimension-line';
  import AdobeAcrobatReaderIcon from '~icons/simple-icons/adobeacrobatreader';
  import MicrosoftWordIcon from '~icons/simple-icons/microsoftword';
  import FileEpubIcon from '~icons/typie/file-epub';
  import FileHwpIcon from '~icons/typie/file-hwp';
  import { createPaginatedLayout, getMaxMargin, mmToPx, pxToMm } from '$lib/editor/utils';
  import { values } from '$lib/editor/values';
  import { graphql } from '$mearie';
  import { PlanUpgradeDialog } from '../plan-upgrade-dialog.svelte';
  import type { LayoutMode } from '@typie/editor';
  import type { PageLayout, PageLayoutPreset } from '$lib/editor/utils';
  import type { DashboardLayout_DocumentExportModal_user$key } from '$mearie';

  type Format = 'DOCX' | 'EPUB' | 'HWP' | 'PDF';

  type Props = {
    user$key: DashboardLayout_DocumentExportModal_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_DocumentExportModal_user on User {
        id

        subscription {
          id
        }
      }
    `),
    () => user$key,
  );

  const app = getAppContext();

  const open = $derived(app.state.exportOpen !== null);
  const slug = $derived(app.state.exportOpen);

  const close = () => {
    app.state.exportOpen = null;
  };

  const format = $derived(app.preference.current.exportFormat);

  $effect(() => {
    if (!user.data.subscription && format !== 'PDF') {
      app.preference.current.exportFormat = 'PDF';
    }
  });
  let useCurrentSettings = $state(false);
  let pageLayout = $state<PageLayout>(createPaginatedLayout('a4'));

  const layoutDisabled = $derived(format === 'EPUB' || useCurrentSettings);

  const formatNotice: Partial<Record<Format, string>> = {
    HWP: '파일 특성상 일부 서식과 페이지 분할이 다르게 표시될 수 있어요.',
    DOCX: '파일 특성상 일부 서식과 페이지 분할이 다르게 표시될 수 있어요.',
    EPUB: '전자책 특성상 문서에 포함된 장식 요소들이 간소화되고, 페이지 레이아웃이 적용되지 않아요.',
  };

  const needUpgrade = $derived(!user.data.subscription);

  const formatItems = $derived(
    [
      { icon: AdobeAcrobatReaderIcon, label: 'PDF (Acrobat)', description: '인쇄와 공유에 적합한 고정 레이아웃', value: 'PDF' as Format },
      { icon: FileHwpIcon, label: 'HWP (한/글)', description: '편집 가능한 한컴오피스 호환 문서', value: 'HWP' as Format },
      { icon: MicrosoftWordIcon, label: 'DOCX (워드)', description: '편집 가능한 Microsoft Word 호환 문서', value: 'DOCX' as Format },
      { icon: FileEpubIcon, label: 'EPUB (전자책)', description: '전자책 리더에서 읽을 수 있는 표준 문서', value: 'EPUB' as Format },
    ].map((item) => ({
      ...item,
      trailing: needUpgrade && item.value !== 'PDF' ? FullAccessBadge : undefined,
    })),
  );

  const documentQuery = createQuery(
    graphql(`
      query DocumentExportModal_Document_Query($slug: String!) {
        document(slug: $slug) {
          id
          layoutMode
        }
      }
    `),
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    () => ({ slug: slug! }),
    () => ({ skip: !open }),
  );

  const [exportDocument, exportDocumentResult] = createMutation(
    graphql(`
      mutation DocumentExportModal_ExportDocument_Mutation($input: ExportDocumentInput!) {
        exportDocument(input: $input) {
          data
          filename
          mimeType
        }
      }
    `),
  );

  $effect(() => {
    if (documentQuery.data) {
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

  const download = (data: string, filename: string, mimeType: string) => {
    const blob = new Blob([Uint8Array.fromBase64(data)], { type: mimeType });
    const url = URL.createObjectURL(blob);

    const a = window.document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();

    URL.revokeObjectURL(url);
  };

  const handleConfirm = async () => {
    if (!documentQuery.data) return;

    try {
      const documentId = documentQuery.data.document.id;
      const layout =
        format === 'EPUB'
          ? null
          : {
              pageWidth: Math.round(pageLayout.pageWidth),
              pageHeight: Math.round(pageLayout.pageHeight),
              pageMarginTop: Math.round(pageLayout.pageMarginTop),
              pageMarginBottom: Math.round(pageLayout.pageMarginBottom),
              pageMarginLeft: Math.round(pageLayout.pageMarginLeft),
              pageMarginRight: Math.round(pageLayout.pageMarginRight),
            };

      const result = await exportDocument({
        input: { documentId, format, layout },
      });

      download(result.exportDocument.data, result.exportDocument.filename, result.exportDocument.mimeType);
      mixpanel.track('export_document', { format });
      close();
    } catch {
      Toast.error('내보내기에 실패했어요. 잠시 후 다시 시도해주세요.');
    }
  };
</script>

<Modal
  style={css.raw({
    maxWidth: '400px',
  })}
  loading={!documentQuery.data}
  onclose={close}
  {open}
>
  {#if documentQuery.data}
    {@const layoutMode = documentQuery.data.document.layoutMode as LayoutMode}
    {@const currentPageEnabled = layoutMode.type === 'paginated'}

    <div class={css({ padding: '20px' })}>
      <h2 class={css({ fontSize: '18px', fontWeight: 'semibold', marginBottom: '16px' })}>파일로 내보내기</h2>

      <div class={flex({ flexDirection: 'column', gap: '16px', paddingY: '8px' })}>
        <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px' })}>
          <div class={css({ fontSize: '13px', color: 'text.subtle' })}>파일 형식</div>
          <Select
            items={formatItems}
            onselect={(value: string) => {
              if (value !== 'PDF' && !user.data.subscription) {
                PlanUpgradeDialog.show({
                  message: `${value as Format} 내보내기는 FULL ACCESS 플랜에서 사용할 수 있어요.`,
                });
                mixpanel.track('open_plan_upgrade_modal', { via: 'document_export', format: value });
                return false;
              }
              app.preference.current.exportFormat = value as Format;
            }}
            value={format}
          />
        </div>

        {#if formatNotice[format]}
          <div
            class={flex({
              alignItems: 'center',
              gap: '8px',
              fontSize: '12px',
              color: 'text.subtle',
              backgroundColor: 'surface.subtle',
              paddingX: '12px',
              paddingY: '8px',
              borderRadius: '6px',
            })}
          >
            <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={InfoIcon} size={14} />
            <span>{formatNotice[format]}</span>
          </div>
        {/if}

        <HorizontalDivider />

        {#if currentPageEnabled}
          <Checkbox disabled={format === 'EPUB'} bind:checked={useCurrentSettings}>
            <span class={css({ fontSize: '14px' })}>현재 페이지 설정 사용</span>
          </Checkbox>
          <HorizontalDivider />
        {/if}

        <div
          class={flex({
            flexDirection: 'column',
            gap: '16px',
            opacity: layoutDisabled ? '50' : '100',
            pointerEvents: layoutDisabled ? 'none' : 'auto',
          })}
        >
          <div class={flex({ flexDirection: 'column', gap: '8px' })}>
            <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px' })}>
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Icon style={css.raw({ color: 'text.faint' })} icon={FileIcon} />
                <div class={css({ fontSize: '13px', color: 'text.subtle' })}>페이지 크기 (mm)</div>
              </div>
              <Select
                disabled={layoutDisabled}
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
                  disabled={layoutDisabled}
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
                  disabled={layoutDisabled}
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
                  disabled={layoutDisabled}
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
                  disabled={layoutDisabled}
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
                  disabled={layoutDisabled}
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
                  disabled={layoutDisabled}
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
        <Button onclick={close} variant="secondary">취소</Button>
        <Button loading={exportDocumentResult.loading} onclick={handleConfirm}>내보내기</Button>
      </div>
    </div>
  {/if}
</Modal>
