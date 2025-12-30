<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, Modal } from '@typie/ui/components';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import { fragment, graphql } from '$graphql';
  import type { DocumentTemplateModal_site } from '$graphql';
  import type { Editor } from '$lib/editor/editor.svelte';

  type Props = {
    $site: DocumentTemplateModal_site;
    editor: Editor;
    focused: boolean;
  };

  let { $site: _site, editor, focused }: Props = $props();

  const site = fragment(
    _site,
    graphql(`
      fragment DocumentTemplateModal_site on Site {
        id

        documentTemplates {
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
    query DocumentTemplateModal_Query($slug: String!) @client {
      document(slug: $slug) {
        id
        snapshot
      }
    }
  `);

  let open = $state(false);

  $effect(() => {
    if (!focused) return;

    const handleOpenTemplateModal = () => {
      open = true;
    };

    window.addEventListener('open-document-template-modal', handleOpenTemplateModal);

    return () => {
      window.removeEventListener('open-document-template-modal', handleOpenTemplateModal);
    };
  });

  const loadTemplate = async (slug: string) => {
    const resp = await query.load({ slug });

    if (resp.document.snapshot) {
      const snapshot = Uint8Array.fromBase64(resp.document.snapshot);
      editor.insertTemplateFragment(snapshot);
    }

    open = false;
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
    {#each $site.documentTemplates as template (template.id)}
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
        에디터 상단 더보기 메뉴에서
        <br />
        기존 문서를 템플릿으로 전환해보세요.
      </div>
    {/each}
  </div>
</Modal>
