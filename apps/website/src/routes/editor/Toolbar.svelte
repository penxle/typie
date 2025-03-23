<script lang="ts">
  import BoldIcon from '~icons/lucide/bold';
  import ImageIcon from '~icons/lucide/image';
  import ItalicIcon from '~icons/lucide/italic';
  import ListIcon from '~icons/lucide/list';
  import ListOrderedIcon from '~icons/lucide/list-ordered';
  import MinusIcon from '~icons/lucide/minus';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import StrikethroughIcon from '~icons/lucide/strikethrough';
  import TextQuoteIcon from '~icons/lucide/text-quote';
  import UnderlineIcon from '~icons/lucide/underline';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor: Ref<Editor>;
  };

  let { editor }: Props = $props();

  const items = $derived([
    {
      id: 'bold',
      label: '굵게',
      icon: BoldIcon,
      active: editor.current.isActive('bold'),
      onclick: () => editor.current.chain().focus().toggleBold().run(),
    },
    {
      id: 'italic',
      label: '기울임',
      icon: ItalicIcon,
      active: editor.current.isActive('italic'),
      onclick: () => editor.current.chain().focus().toggleItalic().run(),
    },
    {
      id: 'underline',
      label: '밑줄',
      icon: UnderlineIcon,
      active: editor.current.isActive('underline'),
      onclick: () => editor.current.chain().focus().toggleUnderline().run(),
    },
    {
      id: 'strike',
      label: '취소선',
      icon: StrikethroughIcon,
      active: editor.current.isActive('strike'),
      onclick: () => editor.current.chain().focus().toggleStrike().run(),
    },
    {
      id: 'bullet_list',
      label: '순서 없는 목록',
      icon: ListIcon,
      active: editor.current.isActive('bullet_list'),
      onclick: () => editor.current.chain().focus().toggleBulletList().run(),
    },
    {
      id: 'ordered_list',
      label: '순서 있는 목록',
      icon: ListOrderedIcon,
      active: editor.current.isActive('ordered_list'),
      onclick: () => editor.current.chain().focus().toggleOrderedList().run(),
    },
    {
      id: 'blockquote',
      label: '인용구',
      icon: TextQuoteIcon,
      active: editor.current.isActive('blockquote'),
      onclick: () => editor.current.chain().focus().setBlockquote().run(),
    },
    {
      id: 'horizontal_rule',
      label: '구분선',
      icon: MinusIcon,
      active: editor.current.isActive('horizontal_rule'),
      onclick: () => editor.current.chain().focus().setHorizontalRule().run(),
    },
    {
      id: 'image',
      label: '이미지',
      icon: ImageIcon,
      active: editor.current.isActive('image'),
      onclick: () => editor.current.chain().focus().setImage().run(),
    },
    {
      id: 'file',
      label: '파일',
      icon: PaperclipIcon,
      active: editor.current.isActive('file'),
      onclick: () => editor.current.chain().focus().setFile().run(),
    },
  ]);
</script>

<div class={flex({ alignItems: 'center', gap: '4px' })}>
  {#each items as { id, label, icon, active, onclick } (id)}
    <button
      class={flex({
        alignItems: 'center',
        gap: '4px',
        borderWidth: '1px',
        borderRadius: '4px',
        paddingX: '4px',
        paddingY: '2px',
        backgroundColor: active ? 'gray.200' : 'transparent',
      })}
      {onclick}
      type="button"
    >
      <Icon {icon} />
      <span class={css({ fontSize: '14px' })}>{label}</span>
    </button>
  {/each}
</div>
