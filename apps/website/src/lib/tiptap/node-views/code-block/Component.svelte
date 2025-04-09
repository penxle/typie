<script lang="ts">
  import { bundledLanguagesInfo } from 'shiki';
  import IconCheck from '~icons/lucide/check';
  import IconCopy from '~icons/lucide/copy';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
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
    backgroundColor: 'gray.50',
    borderRadius: '8px',
    border: '1px solid',
    borderColor: 'gray.200',
    overflow: 'hidden',
    boxShadow: '[0 2px 8px rgba(0, 0, 0, 0.05)]',
  })}
>
  <div
    class={flex({
      position: 'relative',
      alignItems: 'center',
      borderBottomWidth: '1px',
      paddingX: '12px',
      paddingY: '12px',
      backgroundColor: 'gray.100',
    })}
    contentEditable={false}
  >
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <div class={css({ borderRadius: 'full', size: '12px', backgroundColor: '[#FF5F57]' })}></div>
      <div class={css({ borderRadius: 'full', size: '12px', backgroundColor: '[#FFBD2E]' })}></div>
      <div class={css({ borderRadius: 'full', size: '12px', backgroundColor: '[#28C840]' })}></div>
    </div>

    <div class={center({ position: 'absolute', inset: '0' })}>
      {#if editor?.current.isEditable}
        <Menu {node} {updateAttributes} />
      {:else}
        <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'gray.600', userSelect: 'none' })}>
          {languages[attrs.language]}
        </div>
      {/if}
    </div>
  </div>

  <div class={cx('group', css({ position: 'relative' }))}>
    {#if !editor?.current.isEditable}
      <div
        class={css({
          position: 'absolute',
          right: '8px',
          top: '8px',
          opacity: '0',
          transition: 'opacity',
          transitionDuration: '150ms',
          _groupHover: { opacity: '100' },
        })}
        contentEditable={false}
      >
        <button
          class={center({
            size: '28px',
            borderRadius: '6px',
            color: copied ? 'green.500' : 'gray.500',
            backgroundColor: 'white',
            _hover: {
              backgroundColor: 'gray.100',
            },
          })}
          onclick={handleCopy}
          type="button"
        >
          <Icon icon={copied ? IconCheck : IconCopy} size={14} />
        </button>
      </div>
    {/if}

    <NodeViewContentEditable
      style={css.raw({
        paddingX: '16px',
        paddingTop: '12px',
        paddingBottom: '18px',
        fontSize: '14px',
        fontFamily: 'mono',
        overflowX: 'auto',
        whiteSpace: 'pre-wrap',
        backgroundColor: 'white',
        '&:not(:has(.ProseMirror-trailingBreak))': {
          _after: {
            content: '""',
            display: 'inline-block',
          },
        },
      })}
      as="pre"
    />
  </div>
</NodeView>
