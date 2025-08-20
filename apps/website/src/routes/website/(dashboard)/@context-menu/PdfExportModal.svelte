<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex, grid } from '@typie/styled-system/patterns';
  import { Button, Checkbox, HorizontalDivider, Icon, Modal, Select, TextInput } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { clamp, DEFAULT_PAGE_MARGINS, getMaxMargin, PAGE_LAYOUT_OPTIONS } from '@typie/ui/utils';
  import { ExportLayoutMode } from '@/enums';
  import FileIcon from '~icons/lucide/file';
  import PanelBottomDashedIcon from '~icons/lucide/panel-bottom-dashed';
  import PanelLeftDashedIcon from '~icons/lucide/panel-left-dashed';
  import PanelRightDashedIcon from '~icons/lucide/panel-right-dashed';
  import PanelTopDashedIcon from '~icons/lucide/panel-top-dashed';
  import RulerDimensionLineIcon from '~icons/lucide/ruler-dimension-line';
  import type { PageLayoutSettings, PageLayoutSize } from '@typie/ui/utils';

  type Props = {
    open: boolean;
    currentPageLayout?: PageLayoutSettings;
    currentPageEnabled?: boolean;
    onConfirm: (layoutMode: ExportLayoutMode, pageLayout: PageLayoutSettings) => Promise<void>;
    onClose: () => void;
  };

  let { open = $bindable(), currentPageLayout, currentPageEnabled, onConfirm, onClose }: Props = $props();

  const app = getAppContext();

  let isExporting = $state(false);
  let useCurrentSettings = $state(false);
  let pageSize = $state<PageLayoutSize>(app.preference.current.lastPdfPageLayoutSettings.size);
  let margins = $state(app.preference.current.lastPdfPageLayoutSettings.margins);

  $effect(() => {
    if (open) {
      useCurrentSettings = !!currentPageEnabled && !!currentPageLayout;

      if (currentPageLayout && currentPageEnabled) {
        pageSize = currentPageLayout.size;
        margins = { ...currentPageLayout.margins };
      } else {
        pageSize = app.preference.current.lastPdfPageLayoutSettings.size;
        margins = { ...app.preference.current.lastPdfPageLayoutSettings.margins };
      }
    }
  });

  $effect(() => {
    if (useCurrentSettings && currentPageLayout) {
      pageSize = currentPageLayout.size;
      margins = { ...currentPageLayout.margins };
    }
  });

  const handleConfirm = async () => {
    isExporting = true;
    const layoutMode = currentPageEnabled ? ExportLayoutMode.PAGE : ExportLayoutMode.SCROLL;
    if (useCurrentSettings && currentPageLayout) {
      await onConfirm(layoutMode, currentPageLayout);
    } else {
      const pageLayout: PageLayoutSettings = {
        size: pageSize,
        margins,
      };
      app.preference.current.lastPdfPageLayoutSettings = pageLayout;
      await onConfirm(layoutMode, pageLayout);
    }
    isExporting = false;
    onClose();
  };
</script>

<Modal
  style={css.raw({
    maxWidth: '400px',
  })}
  onclose={onClose}
  bind:open
>
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
        <div class={flex({ justifyContent: 'space-between', alignItems: 'center', gap: '32px' })}>
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={FileIcon} />
            <div class={css({ fontSize: '13px', color: 'text.subtle' })}>페이지 크기</div>
          </div>
          <Select
            disabled={useCurrentSettings}
            items={PAGE_LAYOUT_OPTIONS}
            onselect={(value: PageLayoutSize) => {
              pageSize = value;
              margins = { ...DEFAULT_PAGE_MARGINS[value] };
            }}
            value={pageSize}
          />
        </div>

        <div class={flex({ flexDirection: 'column', gap: '8px' })}>
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <Icon style={css.raw({ color: 'text.faint' })} icon={RulerDimensionLineIcon} />
            <div class={css({ fontSize: '13px', color: 'text.subtle' })}>여백 (mm)</div>
          </div>
          <div class={grid({ columns: 2, columnGap: '12px', rowGap: '8px', paddingLeft: '8px' })}>
            <div class={center({ gap: '8px' })}>
              <div class={center({ gap: '4px' })}>
                <Icon style={css.raw({ width: '14px', height: '14px', color: 'text.subtle' })} icon={PanelTopDashedIcon} />
                <div class={css({ fontSize: '12px', color: 'text.subtle' })}>상</div>
              </div>
              <TextInput
                style={css.raw({ width: 'full' })}
                disabled={useCurrentSettings}
                max={String(getMaxMargin('top', pageSize, margins))}
                min="0"
                oninput={(e) => {
                  const target = e.target as HTMLInputElement;
                  const value = clamp(Number(target.value), 0, getMaxMargin('top', pageSize, margins));
                  target.value = String(value);
                  margins.top = value;
                }}
                size="sm"
                type="number"
                value={margins.top}
              />
            </div>
            <div class={center({ gap: '8px' })}>
              <div class={center({ gap: '4px' })}>
                <Icon style={css.raw({ width: '14px', height: '14px', color: 'text.subtle' })} icon={PanelBottomDashedIcon} />
                <div class={css({ fontSize: '12px', color: 'text.subtle' })}>하</div>
              </div>
              <TextInput
                style={css.raw({ width: 'full' })}
                disabled={useCurrentSettings}
                max={String(getMaxMargin('bottom', pageSize, margins))}
                min="0"
                oninput={(e) => {
                  const target = e.target as HTMLInputElement;
                  const value = clamp(Number(target.value), 0, getMaxMargin('bottom', pageSize, margins));
                  target.value = String(value);
                  margins.bottom = value;
                }}
                size="sm"
                type="number"
                value={margins.bottom}
              />
            </div>
            <div class={center({ gap: '8px' })}>
              <div class={center({ gap: '4px' })}>
                <Icon style={css.raw({ width: '14px', height: '14px', color: 'text.subtle' })} icon={PanelLeftDashedIcon} />
                <div class={css({ fontSize: '12px', color: 'text.subtle' })}>좌</div>
              </div>
              <TextInput
                style={css.raw({ width: 'full' })}
                disabled={useCurrentSettings}
                max={String(getMaxMargin('left', pageSize, margins))}
                min="0"
                oninput={(e) => {
                  const target = e.target as HTMLInputElement;
                  const value = clamp(Number(target.value), 0, getMaxMargin('left', pageSize, margins));
                  target.value = String(value);
                  margins.left = value;
                }}
                size="sm"
                type="number"
                value={margins.left}
              />
            </div>
            <div class={center({ gap: '8px' })}>
              <div class={center({ gap: '4px' })}>
                <Icon style={css.raw({ width: '14px', height: '14px', color: 'text.subtle' })} icon={PanelRightDashedIcon} />
                <div class={css({ fontSize: '12px', color: 'text.subtle' })}>우</div>
              </div>
              <TextInput
                style={css.raw({ width: 'full' })}
                disabled={useCurrentSettings}
                max={String(getMaxMargin('right', pageSize, margins))}
                min="0"
                oninput={(e) => {
                  const target = e.target as HTMLInputElement;
                  const value = clamp(Number(target.value), 0, getMaxMargin('right', pageSize, margins));
                  target.value = String(value);
                  margins.right = value;
                }}
                size="sm"
                type="number"
                value={margins.right}
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
</Modal>
