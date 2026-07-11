<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, Modal } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import { getDocumentChannels, loadDocumentSnapshot } from '$lib/sync';
  import { graphql } from '$mearie';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';
  import type { DocumentTemplateModalV2_site$key } from '$mearie';

  type Props = {
    site$key: DocumentTemplateModalV2_site$key;
    editor: Editor | undefined;
    focused: boolean;
  };

  let { site$key, editor, focused }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DocumentTemplateModalV2_site on Site {
        id

        documentTemplates {
          id
          title
        }
      }
    `),
    () => site$key,
  );

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

  const loadTemplate = async (documentId: string) => {
    try {
      const graph = await loadDocumentSnapshot(getDocumentChannels(), documentId);
      editor?.insertTemplateFragment(graph);
    } catch {
      Toast.error('이 템플릿은 아직 사용할 수 없어요.');
    }

    editor?.focus();
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

  <div class={flex({ flexDirection: 'column', padding: '16px' })}>
    {#each site.data.documentTemplates as template (template.id)}
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
        onclick={() => void loadTemplate(template.id)}
        type="button"
      >
        <div class={flex({ alignItems: 'center', gap: '4px' })}>
          <Icon style={css.raw({ color: 'text.faint' })} icon={LayoutTemplateIcon} size={14} />
          <div class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.subtle', lineClamp: '1' })}>
            {template.title}
          </div>
        </div>
        <div class={flex({ alignItems: 'center', gap: '4px', opacity: '0', transition: 'common', _groupHover: { opacity: '100' } })}>
          <div class={css({ fontSize: '13px', color: 'text.faint', whiteSpace: 'nowrap' })}>사용하기</div>
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
