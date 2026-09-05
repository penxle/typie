<script lang="ts" module>
  export type SidebarSectionTab = {
    value: string;
    label: string;
    dropTarget?: string;
  };
</script>

<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import type { Snippet } from 'svelte';

  type Props = {
    actions?: Snippet;
    dividerVisible: boolean;
    height?: number;
    onToggle: () => void;
    open: boolean;
    label?: string;
    tabs?: readonly SidebarSectionTab[];
    activeTab?: string;
    onSelectTab?: (value: string) => void;
  };

  let { actions, dividerVisible, height = $bindable(0), label, onToggle, open, tabs, activeTab, onSelectTab }: Props = $props();

  const activeTabLabel = $derived(tabs?.find((tab) => tab.value === activeTab)?.label);

  const selectTab = (value: string) => {
    if (value === activeTab) {
      onToggle();
      return;
    }

    onSelectTab?.(value);
    if (!open) onToggle();
  };
</script>

{#snippet chevron()}
  <Icon
    style={css.raw({
      opacity: '0',
      transform: open ? 'rotate(90deg)' : 'rotate(0deg)',
      transition: '[opacity 120ms ease, transform 150ms cubic-bezier(0.23, 1, 0.32, 1)]',
      _motionReduce: { transition: '[none]' },
    })}
    aria-hidden="true"
    icon={ChevronRightIcon}
    size={12}
  />
{/snippet}

<div
  class={flex({
    position: 'sticky',
    top: '0',
    zIndex: '1',
    flexShrink: '0',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingX: '12px',
    paddingTop: '8px',
    paddingBottom: '4px',
    backgroundColor: 'surface.canvas',
    _after: {
      content: '""',
      position: 'absolute',
      right: '12px',
      bottom: '0',
      left: '12px',
      height: '1px',
      backgroundColor: 'border.hairline',
      opacity: dividerVisible ? '100' : '0',
      transition: '[opacity 150ms ease]',
    },
  })}
  bind:offsetHeight={height}
>
  {#if tabs}
    <div
      class={flex({
        alignItems: 'center',
        gap: '4px',
        flexGrow: '1',
        minWidth: '0',
        height: '24px',
        paddingX: '4px',
        _supportHover: { '& > button > svg': { opacity: '100' } },
        _focusWithin: { '& > button > svg': { opacity: '100' } },
      })}
    >
      <div class={flex({ alignItems: 'center', gap: '4px', minWidth: '0' })} role="tablist">
        {#each tabs as tab (tab.value)}
          <button
            class={css(
              {
                paddingX: '4px',
                borderRadius: '4px',
                fontSize: '13px',
                fontWeight: 'semibold',
                color: 'text.muted',
                opacity: '50',
                transition: 'common',
                _supportHover: { color: 'text.default', opacity: '100' },
                _focusVisible: { opacity: '100' },
              },
              tab.value === activeTab && { color: 'text.default', opacity: '100' },
            )}
            aria-expanded={tab.value === activeTab ? open : undefined}
            aria-selected={tab.value === activeTab}
            data-drop-target={tab.dropTarget}
            onclick={() => selectTab(tab.value)}
            role="tab"
            type="button"
          >
            {tab.label}
          </button>
        {/each}
      </div>

      <button
        class={flex({
          flexShrink: '0',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: '4px',
          size: '20px',
          color: 'text.muted',
          transition: 'common',
          _supportHover: { color: 'text.default' },
        })}
        aria-expanded={open}
        aria-label={activeTabLabel ? `${activeTabLabel} 열기/닫기` : undefined}
        onclick={onToggle}
        type="button"
      >
        {@render chevron()}
      </button>
    </div>
  {:else}
    <button
      class={flex({
        alignItems: 'center',
        gap: '4px',
        flexGrow: '1',
        minWidth: '0',
        height: '24px',
        paddingX: '8px',
        borderRadius: '4px',
        color: 'text.muted',
        opacity: '80',
        transition: 'common',
        _supportHover: { color: 'text.default', opacity: '100', '& > svg': { opacity: '100' } },
        _focusVisible: { opacity: '100', '& > svg': { opacity: '100' } },
      })}
      aria-expanded={open}
      onclick={onToggle}
      type="button"
    >
      <span class={css({ fontSize: '13px', fontWeight: 'semibold' })}>{label}</span>
      {@render chevron()}
    </button>
  {/if}

  {@render actions?.()}
</div>
