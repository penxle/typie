<script lang="ts">
  import { createFragment, createQuery } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, Modal } from '@typie/ui/components';
  import * as Y from 'yjs';
  import { PostLayoutMode } from '@/enums';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import { graphql } from '$mearie';
  import { YState } from './state.svelte';
  import type { Editor } from '@tiptap/core';
  import type { PageLayout, Ref } from '@typie/ui/utils';
  import type { Editor_Placeholder_site$key } from '$mearie';

  type Props = {
    site$key: Editor_Placeholder_site$key;
    editor: Ref<Editor>;
    doc: Y.Doc;
    focused: boolean;
  };

  let { site$key, editor, doc, focused }: Props = $props();

  const site = createFragment(
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
    () => site$key,
  );

  let templateSlug = $state<string | null>(null);

  const query = createQuery(
    graphql(`
      query Editor_Placeholder_Query($slug: String!) {
        post(slug: $slug) {
          id
          body
          maxWidth
          layoutMode
          pageLayout
          storedMarks
        }
      }
    `),
    () => ({ slug: templateSlug ?? '' }),
    () => ({ skip: !templateSlug }),
  );

  let open = $state(false);

  const maxWidth = new YState<number>(doc, 'maxWidth', 800);
  const layoutMode = new YState<PostLayoutMode>(doc, 'layoutMode', PostLayoutMode.SCROLL);
  const pageLayout = new YState<PageLayout | undefined>(doc, 'pageLayout', undefined);

  $effect(() => {
    if (!focused) return;

    const handleOpenTemplateModal = () => {
      open = true;
    };

    window.addEventListener('open-template-modal', handleOpenTemplateModal);

    return () => {
      window.removeEventListener('open-template-modal', handleOpenTemplateModal);
    };
  });

  $effect(() => {
    if (templateSlug && query.data && !query.loading) {
      maxWidth.current = query.data.post.maxWidth;
      layoutMode.current = query.data.post.layoutMode;
      pageLayout.current = query.data.post.pageLayout;

      editor.current.commands.loadTemplate(query.data.post);

      templateSlug = null;
      open = false;
    }
  });

  const loadTemplate = (slug: string) => {
    templateSlug = slug;
  };
</script>

<Modal style={css.raw({ maxWidth: '400px' })} bind:open>
  <div class={center({ gap: '8px', padding: '12px' })}>
    <div class={center({ gap: '4px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={LayoutTemplateIcon} size={14} />
      <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.faint' })}>템플릿 사용하기</span>
    </div>
  </div>

  <HorizontalDivider />

  <div class={flex({ flexDirection: 'column', paddingX: '24px', paddingY: '16px' })}>
    {#each site.data.templates as template (template.id)}
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
