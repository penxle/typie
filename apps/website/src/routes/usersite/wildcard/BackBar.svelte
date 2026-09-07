<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import { getUsersiteChrome } from '../chrome.svelte';
  import { stuck } from './stuck';

  type Props = {
    label: string;
    compactLabel?: string;
    onBack: () => void;
  };

  let { label, compactLabel = label, onBack }: Props = $props();

  const chrome = getUsersiteChrome();
</script>

<div
  class={css({
    position: 'sticky',
    top: '[var(--usersite-sticky-header-bottom, 0px)]',
    zIndex: '10',
    display: 'flex',
    alignItems: 'center',
    height: '44px',
    borderBottomWidth: '1px',
    borderColor: 'border.hairline',
    backgroundColor: 'surface.default',
    transition: '[box-shadow 150ms ease-out, border-color 150ms ease-out]',
    '&[data-stuck]': { borderColor: 'border.default' },
  })}
  use:stuck={{ onchange: (node, value) => chrome.setStuck(node, value) }}
>
  <button
    class={flex({
      alignItems: 'center',
      gap: '2px',
      height: '32px',
      marginLeft: '-6px',
      paddingLeft: '4px',
      paddingRight: '10px',
      borderRadius: '8px',
      fontSize: '16px',
      fontWeight: 'semibold',
      color: 'text.default',
      transition: 'colors',
      _hover: { color: 'text.muted' },
    })}
    onclick={onBack}
    type="button"
  >
    <Icon icon={ChevronLeftIcon} size={18} />
    <span class={css({ display: { base: 'none', lg: 'inline' } })}>{label}</span>
    <span class={css({ display: { base: 'inline', lg: 'none' } })}>{compactLabel}</span>
  </button>
</div>
