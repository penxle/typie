<script lang="ts">
  import { bundledLanguagesInfo } from 'shiki';
  import IconCheck from '~icons/lucide/check';
  import IconCopy from '~icons/lucide/copy';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import { NodeView, NodeViewContentEditable } from '../../lib';
  import Menu from './Menu.svelte';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, editor, updateAttributes }: Props = $props();

  let attrs = $state(node.attrs);
  $effect(() => {
    attrs = node.attrs;
  });

  const languages = Object.fromEntries([...bundledLanguagesInfo.map((language) => [language.id, language.name]), ['text', 'Plain Text']]);

  let copied = $state(false);
  let timer: NodeJS.Timeout | null = null;

  const handleCopy = async () => {
    await navigator.clipboard.writeText(node.textContent);

    copied = true;

    if (timer) {
      clearTimeout(timer);
    }

    timer = setTimeout(() => {
      copied = false;
    }, 2000);
  };
</script>

<NodeView
  style={css.raw({
    borderWidth: '1px',
    borderRadius: '8px',
    backgroundColor: 'gray.50',
    overflow: 'hidden',
  })}
>
  <div
    class={flex({
      position: 'relative',
      justifyContent: 'space-between',
      alignItems: 'center',
      borderBottomWidth: '1px',
      paddingX: '12px',
      paddingY: '8px',
      backgroundColor: 'gray.100',
    })}
    contentEditable={false}
  >
    <div class={flex({ flexShrink: '0', alignItems: 'center', gap: '6px' })}>
      <div class={css({ borderRadius: 'full', size: '12px', backgroundColor: '[#FF5F57]' })}></div>
      <div class={css({ borderRadius: 'full', size: '12px', backgroundColor: '[#FFBD2E]' })}></div>
      <div class={css({ borderRadius: 'full', size: '12px', backgroundColor: '[#28C840]' })}></div>
    </div>

    <div class={center({ position: 'absolute', inset: '0', pointerEvents: 'none' })}>
      {#if editor?.current.isEditable}
        <Menu {node} {updateAttributes} />
      {:else}
        <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'gray.600', userSelect: 'none' })}>
          {languages[attrs.language]}
        </div>
      {/if}
    </div>

    <button
      class={center({
        flexShrink: '0',
        size: '28px',
        borderRadius: '4px',
        color: copied ? 'green.500' : 'gray.500',
        _hover: { backgroundColor: 'gray.200' },
      })}
      onclick={handleCopy}
      type="button"
    >
      <Icon icon={copied ? IconCheck : IconCopy} size={14} />
    </button>
  </div>

  <NodeViewContentEditable
    style={css.raw({
      paddingX: '16px',
      paddingY: '16px',
      minHeight: '80px',
      fontFamily: 'mono',
      fontSize: '14px',
      backgroundColor: 'white',
      overflowX: 'auto',
      whiteSpace: 'pre',
      tabSize: '4',
      '&:not(:has(.ProseMirror-trailingBreak))': {
        _after: {
          content: '""',
          display: 'inline-block',
        },
      },
    })}
    as="pre"
  />
</NodeView>
