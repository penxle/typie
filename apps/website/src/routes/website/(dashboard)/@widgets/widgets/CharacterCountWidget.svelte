<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import TypeIcon from '~icons/lucide/type';
  import Widget from '../Widget.svelte';
  import { getWidgetContext } from '../widget-context.svelte';

  type Props = {
    widgetId: string;
    data?: Record<string, unknown>;
  };

  let { widgetId, data = {} }: Props = $props();

  const widgetContext = getWidgetContext();
  const { editor } = $derived(widgetContext.env);
  let isCollapsed = $state((data.isCollapsed as boolean) ?? false);

  const toggleCollapse = () => {
    isCollapsed = !isCollapsed;
    widgetContext.updateWidget?.(widgetId, { ...data, isCollapsed });
  };

  $effect(() => {
    if (editor) {
      void editor.characterCountsVersion;
      editor.updateCharacterCounts();
    }
  });

  const counts = $derived(editor?.characterCounts);

  const docCountWithWhitespace = $derived(counts?.docWithWhitespace ?? 0);
  const docCountWithoutWhitespace = $derived(counts?.docWithoutWhitespace ?? 0);
  const docCountWithoutWhitespaceAndPunctuation = $derived(counts?.docWithoutWhitespaceAndPunctuation ?? 0);

  const selectionCountWithWhitespace = $derived(counts?.selectionWithWhitespace ?? 0);
  const selectionCountWithoutWhitespace = $derived(counts?.selectionWithoutWhitespace ?? 0);
  const selectionCountWithoutWhitespaceAndPunctuation = $derived(counts?.selectionWithoutWhitespaceAndPunctuation ?? 0);
</script>

<Widget collapsed={isCollapsed} icon={TypeIcon} title="글자 수">
  {#snippet headerActions()}
    <button
      class={cx(
        'group',
        flex({
          alignItems: 'center',
          height: '26px',
          borderRadius: '6px',
          paddingX: '6px',
          gap: '2px',
          color: 'text.subtle',
          cursor: 'pointer',
          _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
        }),
      )}
      onclick={toggleCollapse}
      type="button"
    >
      {#if isCollapsed}
        <span class={css({ fontSize: '13px', fontWeight: 'normal' })}>
          {comma(docCountWithWhitespace)}자
        </span>
      {/if}
      <Icon icon={isCollapsed ? ChevronDownIcon : ChevronUpIcon} size={14} />
    </button>
  {/snippet}

  <div class={flex({ flexDirection: 'column', gap: '8px' })}>
    <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
      <dt class={css({ color: 'text.faint' })}>공백 포함</dt>
      <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>
        {#if selectionCountWithWhitespace > 0}
          {comma(selectionCountWithWhitespace)}자 /
        {/if}
        {comma(docCountWithWhitespace)}자
      </dd>
    </dl>

    <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
      <dt class={css({ color: 'text.faint' })}>공백 미포함</dt>
      <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>
        {#if selectionCountWithWhitespace > 0}
          {comma(selectionCountWithoutWhitespace)}자 /
        {/if}
        {comma(docCountWithoutWhitespace)}자
      </dd>
    </dl>

    <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
      <dt class={css({ color: 'text.faint' })}>공백/부호 미포함</dt>
      <dd class={css({ fontWeight: 'medium', color: 'text.subtle' })}>
        {#if selectionCountWithWhitespace > 0}
          {comma(selectionCountWithoutWhitespaceAndPunctuation)}자 /
        {/if}
        {comma(docCountWithoutWhitespaceAndPunctuation)}자
      </dd>
    </dl>
  </div>
</Widget>
