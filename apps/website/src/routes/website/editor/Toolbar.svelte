<script lang="ts">
  import BoldIcon from '~icons/lucide/bold';
  import CodeIcon from '~icons/lucide/code';
  import CodeXmlIcon from '~icons/lucide/code-xml';
  import FileUpIcon from '~icons/lucide/file-up';
  import GalleryVerticalEndIcon from '~icons/lucide/gallery-vertical-end';
  import GemIcon from '~icons/lucide/gem';
  import ImageIcon from '~icons/lucide/image';
  import ItalicIcon from '~icons/lucide/italic';
  import LinkIcon from '~icons/lucide/link';
  import ListIcon from '~icons/lucide/list';
  import ListOrderedIcon from '~icons/lucide/list-ordered';
  import MinusIcon from '~icons/lucide/minus';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import StrikethroughIcon from '~icons/lucide/strikethrough';
  import TableIcon from '~icons/lucide/table';
  import TextQuoteIcon from '~icons/lucide/text-quote';
  import UnderlineIcon from '~icons/lucide/underline';
  import { Icon } from '$lib/components';
  import { defaultValues, values } from '$lib/tiptap/values';
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
      id: 'ruby',
      label: '루비',
      icon: GemIcon,
      active: editor.current.isActive('ruby'),
      onclick: () => {
        const text = prompt('루비를 입력하세요', editor.current.getAttributes('ruby').text);
        if (text) {
          editor.current.chain().focus().setRuby(text).run();
        } else {
          editor.current.chain().focus().unsetRuby().run();
        }
      },
    },
    {
      id: 'link',
      label: '링크',
      icon: LinkIcon,
      active: editor.current.isActive('link'),
      onclick: () => {
        const url = prompt('링크를 입력하세요', editor.current.getAttributes('link').href);
        if (url) {
          editor.current.chain().focus().setLink(url).run();
        } else {
          editor.current.chain().focus().unsetLink().run();
        }
      },
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
      id: 'callout',
      label: '콜아웃',
      icon: GalleryVerticalEndIcon,
      active: editor.current.isActive('callout'),
      onclick: () => editor.current.chain().focus().setCallout().run(),
    },
    {
      id: 'table',
      label: '표',
      icon: TableIcon,
      active: editor.current.isActive('table'),
      onclick: () => editor.current.chain().focus().insertTable().run(),
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
    {
      id: 'embed',
      label: '임베드',
      icon: FileUpIcon,
      active: editor.current.isActive('embed'),
      onclick: () => editor.current.chain().focus().setEmbed().run(),
    },
    {
      id: 'code-block',
      label: '코드 블록',
      icon: CodeIcon,
      active: editor.current.isActive('code_block'),
      onclick: () => editor.current.chain().focus().setCodeBlock().run(),
    },
    {
      id: 'html-block',
      label: 'HTML 블록',
      icon: CodeXmlIcon,
      active: editor.current.isActive('html_block'),
      onclick: () => editor.current.chain().focus().setHtmlBlock().run(),
    },
  ]);
</script>

<div class={flex({ justifyContent: 'center', alignItems: 'center', gap: '4px', flexWrap: 'wrap', width: 'full', maxWidth: '1000px' })}>
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

  <select
    class={css({ borderWidth: '1px', borderRadius: '4px', paddingX: '4px', paddingY: '2px', fontSize: '14px' })}
    onchange={({ currentTarget }) =>
      editor.current
        .chain()
        .focus()
        .setFontFamily(currentTarget.value as never)
        .run()}
  >
    {#each values.fontFamily as { label, value } (value)}
      <option selected={(editor.current.getAttributes('font_family').value ?? defaultValues.fontFamily) === value} {value}>
        {label}
      </option>
    {/each}
  </select>

  <select
    class={css({ borderWidth: '1px', borderRadius: '4px', paddingX: '4px', paddingY: '2px', fontSize: '14px' })}
    onchange={({ currentTarget }) =>
      editor.current
        .chain()
        .focus()
        .setFontSize(Number(currentTarget.value) as never)
        .run()}
  >
    {#each values.fontSize as { label, value } (value)}
      <option selected={(editor.current.getAttributes('font_size').value ?? defaultValues.fontSize) === value} {value}>
        {label}
      </option>
    {/each}
  </select>

  <select
    class={css({ borderWidth: '1px', borderRadius: '4px', paddingX: '4px', paddingY: '2px', fontSize: '14px' })}
    onchange={({ currentTarget }) =>
      editor.current
        .chain()
        .focus()
        .setParagraphLineHeight(Number(currentTarget.value) as never)
        .run()}
  >
    {#each values.lineHeight as { label, value } (value)}
      <option selected={(editor.current.getAttributes('paragraph').lineHeight ?? defaultValues.lineHeight) === value} {value}>
        {label}
      </option>
    {/each}
  </select>

  <select
    class={css({ borderWidth: '1px', borderRadius: '4px', paddingX: '4px', paddingY: '2px', fontSize: '14px' })}
    onchange={({ currentTarget }) =>
      editor.current
        .chain()
        .focus()
        .setParagraphLetterSpacing(Number(currentTarget.value) as never)
        .run()}
  >
    {#each values.letterSpacing as { label, value } (value)}
      <option selected={(editor.current.getAttributes('paragraph').letterSpacing ?? defaultValues.letterSpacing) === value} {value}>
        {label}
      </option>
    {/each}
  </select>

  <select
    class={css({ borderWidth: '1px', borderRadius: '4px', paddingX: '4px', paddingY: '2px', fontSize: '14px' })}
    onchange={({ currentTarget }) =>
      editor.current
        .chain()
        .focus()
        .setParagraphTextAlign(currentTarget.value as never)
        .run()}
  >
    {#each values.textAlign as { label, value } (value)}
      <option selected={(editor.current.getAttributes('paragraph').textAlign ?? defaultValues.textAlign) === value} {value}>
        {label}
      </option>
    {/each}
  </select>

  <select
    class={css({ borderWidth: '1px', borderRadius: '4px', paddingX: '4px', paddingY: '2px', fontSize: '14px' })}
    onchange={({ currentTarget }) =>
      editor.current
        .chain()
        .focus()
        .setBodyMaxWidth(Number(currentTarget.value) as never)
        .run()}
  >
    {#each values.maxWidth as { label, value } (value)}
      <option selected={(editor.current.getAttributes('body').maxWidth ?? defaultValues.maxWidth) === value} {value}>
        {label}
      </option>
    {/each}
  </select>

  <select
    class={css({ borderWidth: '1px', borderRadius: '4px', paddingX: '4px', paddingY: '2px', fontSize: '14px' })}
    onchange={({ currentTarget }) =>
      editor.current
        .chain()
        .focus()
        .setBodyBlockGap(Number(currentTarget.value) as never)
        .run()}
  >
    {#each values.blockGap as { label, value } (value)}
      <option selected={(editor.current.getAttributes('body').blockGap ?? defaultValues.blockGap) === value} {value}>
        {label}
      </option>
    {/each}
  </select>

  <select
    class={css({ borderWidth: '1px', borderRadius: '4px', paddingX: '4px', paddingY: '2px', fontSize: '14px' })}
    onchange={({ currentTarget }) =>
      editor.current
        .chain()
        .focus()
        .setBodyParagraphIndent(Number(currentTarget.value) as never)
        .run()}
  >
    {#each values.paragraphIndent as { label, value } (value)}
      <option selected={(editor.current.getAttributes('body').paragraphIndent ?? defaultValues.paragraphIndent) === value} {value}>
        {label}
      </option>
    {/each}
  </select>

  <input
    class={css({ borderWidth: '1px', borderRadius: '4px', paddingX: '4px', paddingY: '2px', fontSize: '14px' })}
    onchange={({ currentTarget }) => editor.current.chain().focus().setFontColor(currentTarget.value).run()}
    type="color"
  />
</div>
