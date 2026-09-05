<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { onDestroy } from 'svelte';
  import CheckIcon from '~icons/lucide/check';
  import CopyIcon from '~icons/lucide/copy';

  type Props = {
    id: string;
    label: string;
  };

  let { id, label }: Props = $props();

  const COPY_FEEDBACK_MS = 2000;

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(id);
      copied = true;
      if (copyTimer !== null) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => {
        copied = false;
        copyTimer = null;
      }, COPY_FEEDBACK_MS);
    } catch {
      Toast.error('ID를 복사하지 못했어요');
    }
  };

  onDestroy(() => {
    if (copyTimer !== null) clearTimeout(copyTimer);
  });
</script>

<button
  class={flex({
    alignItems: 'center',
    gap: '4px',
    width: 'fit',
    cursor: 'pointer',
    fontSize: '11px',
    color: 'text.hint',
    transition: 'common',
    _hover: { color: 'text.muted' },
    _focus: { color: 'text.muted' },
    outlineWidth: '0',
  })}
  aria-label={copied ? `${label}, 복사됨` : label}
  onclick={() => void copy()}
  role="menuitem"
  tabindex="-1"
  type="button"
>
  <Icon icon={copied ? CheckIcon : CopyIcon} size={12} />
  {label}
</button>
