<script lang="ts">
  import { tooltip } from '@typie/ui/actions';
  import { Tooltip } from '@typie/ui/components';
  import type { Placement } from '@floating-ui/dom';

  type Props = {
    firstMessage?: string | null;
    firstDelay?: number;
    firstForce?: boolean;
    firstKeepOnClick?: boolean;
    firstPlacement?: Placement;
    firstArrow?: boolean;
    secondMessage?: string | null;
    secondDelay?: number;
    thirdMessage?: string | null;
    thirdDelay?: number;
    wrapperMessage?: string;
    wrapperEnabled?: boolean;
    wrapperKeepShowing?: boolean;
    wrapperUsesSnippet?: boolean;
    wrapperPlacement?: Placement;
    browserLayout?: boolean;
  };

  let {
    firstMessage = 'First tooltip',
    firstDelay = 500,
    firstForce,
    firstKeepOnClick = false,
    firstPlacement = 'bottom',
    firstArrow = true,
    secondMessage = 'Second tooltip',
    secondDelay = 500,
    thirdMessage = 'Third tooltip',
    thirdDelay = 0,
    wrapperMessage = 'Wrapper tooltip',
    wrapperEnabled = true,
    wrapperKeepShowing = false,
    wrapperUsesSnippet = false,
    wrapperPlacement = 'bottom',
    browserLayout = false,
  }: Props = $props();

  let motionFarMessage = $state('Motion far');
</script>

{#snippet wrapperSnippet()}
  <strong data-testid="wrapper-snippet">Wrapper snippet</strong>
{/snippet}

{#snippet actionSnippet()}
  <strong data-testid="action-snippet">Action snippet</strong>
{/snippet}

{#snippet tallTooltipSnippet()}
  <span style="display: block; width: 100px">
    Tall line one
    <br />
    Tall line two
    <br />
    Tall line three
    <br />
    Tall line four
    <br />
    Tall line five
    <br />
    Tall line six
  </span>
{/snippet}

{#snippet wrapperFirstSnippet()}
  <span data-testid="motion-wrapper-first-content">A wrapper tooltip that wraps onto multiple lines</span>
{/snippet}

{#snippet wrapperSecondSnippet()}
  <span data-testid="motion-wrapper-second-content">Another nearby wrapper tooltip with differently wrapped content</span>
{/snippet}

{#snippet wrapperStyledFirstSnippet()}
  <span>Styled wrapper first</span>
{/snippet}

{#snippet wrapperStyledSecondSnippet()}
  <span>Styled wrapper second</span>
{/snippet}

<div class:browser-layout={browserLayout} data-testid="tooltip-test-root">
  <button
    data-testid="action-first"
    type="button"
    use:tooltip={{
      message: firstMessage,
      delay: firstDelay,
      force: firstForce,
      keepOnClick: firstKeepOnClick,
      placement: firstPlacement,
      arrow: firstArrow,
    }}
  >
    First trigger
  </button>

  <button data-testid="action-second" type="button" use:tooltip={{ message: secondMessage, delay: secondDelay }}>Second trigger</button>

  <button data-testid="action-third" type="button" use:tooltip={{ message: thirdMessage, delay: thirdDelay }}>Third trigger</button>

  <button data-testid="action-snippet-trigger" type="button" use:tooltip={{ message: actionSnippet, delay: 1000, placement: 'right' }}>
    Action snippet trigger
  </button>

  <Tooltip
    style={{ display: 'inline-flex' }}
    enabled={wrapperEnabled}
    keepShowing={wrapperKeepShowing}
    message={wrapperUsesSnippet ? wrapperSnippet : wrapperMessage}
    placement={wrapperPlacement}
    tooltipStyle={{ maxWidth: '[240px]' }}
  >
    <button data-testid="wrapper" type="button">Wrapper trigger</button>
  </Tooltip>

  {#if browserLayout}
    <div class="motion-scroll-region" data-testid="motion-scroll-region">
      <div class="motion-scroll-content">
        <button
          class="motion-anchor motion-anchor-first"
          data-testid="motion-first"
          type="button"
          use:tooltip={{ message: '굵게', keys: ['Mod', 'B'], delay: 0, placement: 'bottom' }}
        >
          Motion first
        </button>
        <button
          class="motion-anchor motion-anchor-second"
          data-testid="motion-second"
          type="button"
          use:tooltip={{ message: '기울임', keys: ['Mod', 'I'], delay: 0, placement: 'bottom' }}
        >
          Motion second
        </button>
        <button
          class="motion-anchor motion-anchor-third"
          data-testid="motion-third"
          type="button"
          use:tooltip={{ message: '밑줄', keys: ['Mod', 'U'], delay: 0, placement: 'bottom' }}
        >
          Motion third
        </button>
        <button
          class="motion-toolbar-anchor motion-toolbar-lock"
          data-testid="motion-toolbar-lock"
          type="button"
          use:tooltip={{ message: '편집 잠금', delay: 0, placement: 'bottom' }}
        >
          Lock
        </button>
        <button
          class="motion-toolbar-anchor motion-toolbar-zen"
          data-testid="motion-toolbar-zen"
          type="button"
          use:tooltip={{ message: '집중 모드 켜기', keys: ['Mod', 'Shift', 'M'], delay: 0, placement: 'bottom' }}
        >
          Zen
        </button>
        <button
          class="motion-toolbar-anchor motion-toolbar-close"
          data-testid="motion-toolbar-close"
          type="button"
          use:tooltip={{ message: '창 닫기', delay: 0, placement: 'bottom' }}
        >
          Close
        </button>
        <button
          class="motion-anchor motion-anchor-side"
          data-testid="motion-side"
          type="button"
          use:tooltip={{ message: 'Motion side', delay: 0, placement: 'top' }}
        >
          Motion side
        </button>
        <button
          class="motion-anchor motion-prism-list"
          data-testid="motion-prism-list"
          type="button"
          use:tooltip={{ message: '대화 목록 열기', delay: 0, placement: 'bottom' }}
        >
          Prism list
        </button>
        <button
          class="motion-anchor motion-prism-close"
          data-testid="motion-prism-close"
          type="button"
          use:tooltip={{ message: 'PRISM 닫기', keys: ['Mod', 'E'], delay: 0, placement: 'bottom' }}
        >
          Prism close
        </button>
      </div>
    </div>

    <button
      class="motion-anchor motion-anchor-far"
      data-testid="motion-far"
      type="button"
      use:tooltip={{ message: motionFarMessage, delay: 0, placement: 'bottom' }}
    >
      Motion far
    </button>

    <button data-testid="motion-far-update" onclick={() => (motionFarMessage = 'Updated far tooltip')} type="button">Update far</button>

    <div class="motion-wrapper-first-position">
      <Tooltip style={{ display: 'inline-flex' }} message={wrapperFirstSnippet} placement="bottom">
        <button class="motion-anchor" data-testid="motion-wrapper-first" type="button">Wrapper first</button>
      </Tooltip>
    </div>

    <div class="motion-wrapper-second-position">
      <Tooltip style={{ display: 'inline-flex' }} message={wrapperSecondSnippet} placement="bottom">
        <button class="motion-anchor" data-testid="motion-wrapper-second" type="button">Wrapper second</button>
      </Tooltip>
    </div>

    <div class="motion-wrapper-styled-first-position">
      <Tooltip
        style={{ display: 'inline-flex' }}
        message={wrapperStyledFirstSnippet}
        placement="bottom"
        tooltipStyle={{ paddingX: '[4px]' }}
      >
        <button class="motion-anchor" data-testid="motion-wrapper-styled-first" type="button">Styled wrapper first</button>
      </Tooltip>
    </div>

    <div class="motion-wrapper-styled-second-position">
      <Tooltip
        style={{ display: 'inline-flex' }}
        message={wrapperStyledSecondSnippet}
        placement="bottom"
        tooltipStyle={{ paddingX: '[12px]' }}
      >
        <button class="motion-anchor" data-testid="motion-wrapper-styled-second" type="button">Styled wrapper second</button>
      </Tooltip>
    </div>

    <button
      class="motion-anchor motion-anchor-bottom-first"
      data-testid="motion-bottom-first"
      type="button"
      use:tooltip={{ message: 'Bottom short', delay: 0, placement: 'bottom' }}
    >
      Bottom first
    </button>

    <div class="motion-bottom-tall-position">
      <Tooltip style={{ display: 'inline-flex' }} message={tallTooltipSnippet} placement="bottom">
        <button class="motion-anchor" data-testid="motion-bottom-tall" type="button">Bottom tall</button>
      </Tooltip>
    </div>

    <dialog class="motion-dialog" open>
      <Tooltip message="Dialog tooltip" placement="bottom">
        <button class="motion-anchor" data-testid="motion-dialog" type="button">Dialog tooltip</button>
      </Tooltip>
    </dialog>
  {/if}
</div>

<style>
  :global(body:has([data-testid='tooltip-test-root'].browser-layout)) {
    min-height: 720px;
    margin: 0;
  }

  :global([data-tooltip-surface]:has([data-testid='motion-wrapper-first-content'])) {
    width: 120px;
  }

  :global([data-tooltip-surface]:has([data-testid='motion-wrapper-second-content'])) {
    width: 240px;
  }

  .browser-layout {
    position: relative;
    min-height: 720px;
  }

  .motion-scroll-region {
    position: absolute;
    top: 80px;
    left: 80px;
    width: 360px;
    height: 160px;
    overflow: auto;
    border: 1px solid transparent;
  }

  .motion-scroll-content {
    position: relative;
    width: 360px;
    height: 520px;
  }

  .motion-anchor {
    width: 96px;
    height: 32px;
  }

  .motion-anchor-first,
  .motion-anchor-second,
  .motion-anchor-third,
  .motion-anchor-side,
  .motion-prism-list,
  .motion-prism-close {
    position: absolute;
    top: 56px;
  }

  .motion-toolbar-anchor {
    position: fixed;
    top: 80px;
    width: 24px;
    height: 24px;
  }

  .motion-toolbar-lock {
    right: 56px;
  }

  .motion-toolbar-zen {
    right: 32px;
  }

  .motion-toolbar-close {
    right: 8px;
  }

  .motion-anchor-first {
    left: 20px;
  }

  .motion-anchor-second {
    left: 140px;
  }

  .motion-anchor-third {
    left: 260px;
  }

  .motion-anchor-side {
    left: 20px;
  }

  .motion-prism-list {
    top: 8px;
    left: 220px;
    width: 32px;
  }

  .motion-prism-close {
    top: 8px;
    left: 260px;
    width: 32px;
  }

  .motion-anchor-far {
    position: absolute;
    top: 136px;
    left: 320px;
  }

  .motion-anchor-bottom-first {
    position: fixed;
    bottom: 100px;
    left: 80px;
  }

  .motion-wrapper-first-position,
  .motion-wrapper-second-position,
  .motion-wrapper-styled-first-position,
  .motion-wrapper-styled-second-position,
  .motion-bottom-tall-position {
    position: absolute;
  }

  .motion-wrapper-first-position {
    top: 300px;
    left: 80px;
  }

  .motion-wrapper-second-position {
    top: 300px;
    left: 200px;
  }

  .motion-wrapper-styled-first-position {
    top: 500px;
    left: 80px;
  }

  .motion-wrapper-styled-second-position {
    top: 500px;
    left: 200px;
  }

  .motion-bottom-tall-position {
    position: fixed;
    bottom: 100px;
    left: 80px;
  }

  .motion-dialog {
    position: absolute;
    top: 360px;
    left: 80px;
    width: 220px;
    height: 100px;
    margin: 0;
  }
</style>
