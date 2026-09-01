<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, Marquee, Menu, MenuItem } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import { getDocumentChannels, loadDocumentSnapshot } from '$lib/sync';
  import { graphql } from '$mearie';
  import EntityIcon from '../../@context-menu/EntityIcon.svelte';
  import { SubscribeModal } from '../../@subscription/subscribe-modal.svelte';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';
  import type { DocumentTemplateMenuV2_site$key } from '$mearie';

  type Props = {
    site$key: DocumentTemplateMenuV2_site$key;
    editor: Editor | undefined;
  };

  let { site$key, editor }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DocumentTemplateMenuV2_site on Site {
        id

        documentTemplates {
          id
          title

          entity {
            id
            ...EntityIcon_entity
          }
        }
      }
    `),
    () => site$key,
  );

  const getTemplateItem = (element: HTMLElement) => element.closest<HTMLElement>('[role="menuitem"]');

  const loadTemplate = async (documentId: string) => {
    if (!SubscribeModal.gate('document_template')) {
      return;
    }

    try {
      const graph = await loadDocumentSnapshot(getDocumentChannels(), documentId);
      editor?.insertTemplateFragment(graph);
    } catch {
      Toast.error('템플릿을 불러오지 못했어요.');
    }

    editor?.focus();
  };
</script>

<Menu
  style={css.raw({
    display: 'inline-flex',
    alignItems: 'center',
    gap: '4px',
    color: '[inherit]',
    font: '[inherit]',
    textAlign: '[inherit]',
    transition: 'common',
    pointerEvents: 'auto',
    _hover: { color: 'text.faint' },
    _expanded: { color: 'text.faint' },
  })}
  buttonAriaLabel="템플릿 불러오기"
  listStyle={css.raw({ width: '280px', maxWidth: '[calc(100vw - 16px)]', maxHeight: '[min(268px, var(--floating-available-height))]' })}
  placement="bottom-start"
  scrollbarLabel="템플릿 목록 세로 스크롤"
>
  {#snippet button()}
    <Icon style={css.raw({ size: '[1em]' })} icon={LayoutTemplateIcon} />
    <span>템플릿 불러오기</span>
  {/snippet}

  {#each site.data.documentTemplates as template (template.id)}
    <MenuItem
      style={css.raw({ minHeight: '36px', paddingX: '10px', paddingY: '6px', fontSize: '14px' })}
      onclick={() => void loadTemplate(template.id)}
    >
      {#snippet prefix()}
        <EntityIcon entity$key={template.entity} size={14} />
      {/snippet}
      <Marquee class={css({ flex: '1', minWidth: '0' })} fogSize={16} getTrigger={getTemplateItem} text={template.title} />
      {#snippet suffix()}
        <span
          class={flex({
            alignItems: 'center',
            flexShrink: '0',
            gap: '4px',
            color: 'text.faint',
            opacity: '0',
            transition: 'common',
            _groupFocus: { opacity: '100' },
          })}
        >
          <span class={css({ fontSize: '13px', whiteSpace: 'nowrap' })}>사용하기</span>
          <Icon icon={ChevronRightIcon} size={16} />
        </span>
      {/snippet}
    </MenuItem>
  {:else}
    <div class={css({ paddingX: '10px', paddingY: '8px', fontSize: '13px', color: 'text.faint', textAlign: 'left' })} role="presentation">
      아직 템플릿이 없어요.
    </div>
  {/each}
</Menu>
