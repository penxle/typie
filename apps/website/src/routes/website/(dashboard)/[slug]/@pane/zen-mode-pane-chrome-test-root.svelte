<script lang="ts">
  import { PANE_CHROME_EXPANSION_EASING, paneChromeExpansionTiming, setupZenModePaneChrome } from './zen-mode-pane-chrome.svelte';
  import ZenModePaneChromeEffects from './ZenModePaneChromeEffects.svelte';
  import ZenModePaneChromeSegment from './ZenModePaneChromeSegment.svelte';

  type Props = { initialActive?: boolean; reclassifyActionsOnEntry?: boolean; toolbarRows?: 1 | 2 };

  let { initialActive = true, reclassifyActionsOnEntry = false, toolbarRows = 1 }: Props = $props();
  let active = $state(initialActive);
  let headerPointerDownCount = $state(0);
  let actionsMounted = $state(true);
  let actionsMenuOpen = $state(false);
  const chrome = setupZenModePaneChrome({ active: () => active, focused: () => true });
  const registerRoot = chrome.registerRoot;
  const registerHeader = chrome.registerHeaderLane;
  const registerIdentity = (node: HTMLElement) => chrome.registerSegment('identity', node);
  const registerActions = (node: HTMLElement) => chrome.registerSegment('actions', node);
  const registerToolbar = chrome.registerToolbarLane;
  const zoomAttachment = chrome.attachmentHandle();
  const expansionTiming = $derived(paneChromeExpansionTiming(chrome.phase === 'expanding' ? chrome.expansionPace : 'standard'));

  export function setActive(next: boolean): void {
    active = next;
  }

  export function holdActions(): void {
    chrome.hold('actions', 'hover');
  }

  export function holdChromeAttachment(): void {
    zoomAttachment.hold();
  }

  export function releaseChromeAttachment(): void {
    zoomAttachment.release();
  }

  export function holdChromeAttachmentAt(clientX: number, clientY: number): void {
    zoomAttachment.hold(new PointerEvent('pointermove', { clientX, clientY, pointerType: 'mouse' }));
  }

  export function removeActions(): void {
    actionsMounted = false;
  }

  export function openActionsMenu(): void {
    actionsMenuOpen = true;
  }

  export function closeActionsMenu(): void {
    actionsMenuOpen = false;
  }
</script>

<div
  style="position: relative; width: 600px; height: 240px"
  data-chrome-root
  onpointerleave={() => chrome.handlePointerLeave()}
  onpointermove={(event) => chrome.handlePointerMove(event)}
  role="presentation"
  use:registerRoot
>
  <button style="position: absolute; inset: 0" data-editor-target type="button">Editor</button>
  <div
    style="position: absolute; top: 0; right: 0; left: 0; height: 37px"
    style:pointer-events="none"
    aria-label="Pane header test"
    data-chrome-header
    onpointerdown={() => (headerPointerDownCount += 1)}
    role="region"
    use:registerHeader
  >
    <ZenModePaneChromeEffects lane="header" />
    <div
      style="position: absolute; inset: 0"
      style:--zen-pane-chrome-foreground-radius={`${chrome.foregroundRevealRadius('header')}px`}
      style:clip-path={chrome.headerLaneInteractionClip()}
      style:cursor="grab"
      style:pointer-events={chrome.isHeaderLaneInteractive() ? 'auto' : 'none'}
      style:transition={`clip-path ${expansionTiming.backgroundExpandMs}ms ${PANE_CHROME_EXPANSION_EASING}`}
      aria-hidden="true"
      data-chrome-header-foreground-hit
    ></div>
    <ZenModePaneChromeSegment
      style={`position: absolute; top: 0; left: 0; width: ${reclassifyActionsOnEntry && active ? '500px' : '140px'}; height: 36px`}
      {active}
      contentCursor="grab"
      data-chrome-identity
      register={registerIdentity}
      segment="identity"
    >
      Identity
    </ZenModePaneChromeSegment>
    {#if actionsMounted}
      <ZenModePaneChromeSegment
        style="position: absolute; top: 0; right: 0; width: 180px; height: 36px"
        {active}
        contentCursor="grab"
        data-chrome-actions
        register={registerActions}
        segment="actions"
      >
        <button
          aria-expanded={actionsMenuOpen}
          aria-haspopup="menu"
          data-chrome-focus-toggle
          onclick={(event) => {
            chrome.prepareEntryReveal('actions', event);
            active = !active;
          }}
          type="button"
        >
          Actions
        </button>
      </ZenModePaneChromeSegment>
    {/if}
  </div>
  <div style="position: absolute; top: 37px; right: 0; left: 0; pointer-events: none" data-chrome-toolbar-lane use:registerToolbar>
    <ZenModePaneChromeEffects lane="toolbar" toolbarSeparatorOffsets={toolbarRows === 2 ? [40] : []} />
    <ZenModePaneChromeSegment {active} data-chrome-toolbar segment="toolbar">
      <div style:height="41px" data-chrome-toolbar-content>
        <button data-chrome-toolbar-action type="button">Toolbar</button>
      </div>
      {#if toolbarRows === 2}
        <div style="height: 41px" data-chrome-toolbar-expanded-content>Expanded toolbar</div>
      {/if}
    </ZenModePaneChromeSegment>
  </div>
  <div
    style="position: absolute; top: 0; right: 0; width: 180px; height: 36px"
    data-chrome-reveal-exclusion
    data-pane-chrome-reveal-exclusion
  ></div>
</div>

<output data-chrome-phase>{chrome.phase}</output>
<output data-chrome-occlusion>{chrome.topOcclusion}</output>
<output data-chrome-header-inset>{chrome.headerInset}</output>
<output data-chrome-zoom-top>{chrome.floatingZoomTopInset}</output>
<output data-chrome-zoom-inset>{chrome.floatingZoomInset}</output>
<output data-chrome-zoom-ready>{zoomAttachment.discoverable()}</output>
<output data-chrome-zoom-attached>{zoomAttachment.attached()}</output>
<output data-chrome-header-pointer>{JSON.stringify(chrome.pointerInLane('header'))}</output>
<output data-chrome-toolbar-pointer>{JSON.stringify(chrome.pointerInLane('toolbar'))}</output>
<output data-chrome-header-pointer-down-count>{headerPointerDownCount}</output>
