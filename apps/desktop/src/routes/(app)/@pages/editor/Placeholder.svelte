<script lang="ts">
  import { Mark } from '@tiptap/pm/model';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, Modal } from '@typie/ui/components';
  import { isBodyEmpty } from '@typie/ui/tiptap';
  import * as Y from 'yjs';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import ShapesIcon from '~icons/lucide/shapes';
  import { fragment, graphql } from '$graphql';
  import { YState } from './state.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';
  import type { Editor_Placeholder_site } from '$graphql';

  type Props = {
    $site: Editor_Placeholder_site;
    editor: Ref<Editor>;
    doc: Y.Doc;
  };

  let { $site: _site, editor, doc }: Props = $props();

  const site = fragment(
    _site,
    graphql(`
      fragment Editor_Placeholder_site on Site {
        id

        templates {
          id
          title

          entity {
            id
            slug
          }
        }
      }
    `),
  );

  const query = graphql(`
    query Editor_Placeholder_Query($slug: String!) @client {
      post(slug: $slug) {
        id
        body
        maxWidth
        storedMarks
      }
    }
  `);

  let open = $state(false);

  const maxWidth = new YState<number>(doc, 'maxWidth', 800);

  const emptyBody = $derived(isBodyEmpty(editor.current.state));

  const paragraphIndent = $derived.by(() => {
    const { doc } = editor.current.state;
    const body = doc.child(0);
    return body.attrs.paragraphIndent;
  });

  const loadTemplate = async (slug: string) => {
    const resp = await query.load({ slug });

    maxWidth.current = resp.post.maxWidth;
    editor.current
      .chain()
      .focus(2)
      .setContent(resp.post.body)
      .setTextSelection(2)
      .command(({ tr, dispatch }) => {
        tr.setStoredMarks(resp.post.storedMarks.map((mark: unknown) => Mark.fromJSON(editor.current.state.schema, mark)));
        dispatch?.(tr);
        return true;
      })
      .run();

    open = false;
  };
</script>

{#if emptyBody}
  <div
    style:padding-left={`${paragraphIndent}em`}
    class={flex({
      flexDirection: 'column',
      gap: '4px',
      width: 'full',
      maxWidth: 'var(--prosemirror-max-width)',
      color: 'text.disabled',
      lineHeight: '[1.6]',
      pointerEvents: 'none',
    })}
  >
    <div class={css({ fontFamily: 'ui' })}>내용을 입력하거나 /를 입력해 블록 삽입하기...</div>

    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <div>혹은</div>
      <button
        class={flex({
          alignItems: 'center',
          gap: '4px',
          transition: 'common',
          pointerEvents: 'auto',
          _hover: { color: 'text.faint' },
        })}
        onclick={() => (open = true)}
        type="button"
      >
        <Icon icon={ShapesIcon} size={16} />
        <div>템플릿 사용하기</div>
      </button>
    </div>
  </div>
{/if}

<Modal style={css.raw({ maxWidth: '400px' })} bind:open>
  <div class={center({ gap: '8px', padding: '12px' })}>
    <div class={center({ gap: '4px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={ShapesIcon} size={14} />
      <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.faint' })}>템플릿 사용하기</span>
    </div>
  </div>

  <HorizontalDivider />

  <div class={flex({ flexDirection: 'column', paddingX: '24px', paddingY: '16px' })}>
    {#each $site.templates as template (template.id)}
      <button
        class={cx(
          'group',
          flex({
            justifyContent: 'space-between',
            alignItems: 'center',
            gap: '4px',
            borderRadius: '4px',
            padding: '12px',
            textAlign: 'left',
            transition: 'common',
            _hover: { backgroundColor: 'surface.muted' },
          }),
        )}
        onclick={() => loadTemplate(template.entity.slug)}
        type="button"
      >
        <div class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.subtle' })}>{template.title}</div>
        <div class={flex({ alignItems: 'center', gap: '4px', opacity: '0', transition: 'common', _groupHover: { opacity: '100' } })}>
          <div class={css({ fontSize: '13px', color: 'text.faint' })}>사용하기</div>
          <Icon style={css.raw({ color: 'text.faint' })} icon={ChevronRightIcon} size={16} />
        </div>
      </button>
    {:else}
      <div class={center({ fontSize: '13px', color: 'text.faint', textAlign: 'center' })}>
        아직 템플릿이 없어요.
        <br />
        <br />
        에디터 우상단 더보기 메뉴에서
        <br />
        기존 포스트를 템플릿으로 전환해보세요.
      </div>
    {/each}
  </div>
</Modal>
