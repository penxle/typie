<script lang="ts">
  import { createFragment, createQuery } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import { graphql } from '$mearie';
  import { PlanUpgradeDialog } from '../plan-upgrade-dialog.svelte';
  import type { Editor } from '$lib/editor/editor.svelte';
  import type { DocumentTemplateModal_site$key } from '$mearie';

  type Props = {
    site$key: DocumentTemplateModal_site$key;
    editor: Editor;
    focused: boolean;
  };

  let { site$key, editor, focused }: Props = $props();

  const app = getAppContext();

  const site = createFragment(
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
    () => site$key,
  );

  let templateSlug = $state<string | null>(null);

  const query = createQuery(
    graphql(`
      query DocumentTemplateModal_Query($slug: String!) {
        document(slug: $slug) {
          id
          snapshot
        }
      }
    `),
    () => ({ slug: templateSlug ?? '' }),
    () => ({ skip: !templateSlug }),
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

  $effect(() => {
    if (!(templateSlug && query.data) || query.loading) {
      return;
    }

    if (query.data.document.snapshot) {
      const snapshot = Uint8Array.fromBase64(query.data.document.snapshot);
      editor.insertTemplateFragment(snapshot);
    }
    templateSlug = null;
    open = false;
  });

  const loadTemplate = (slug: string) => {
    if (!app.state.subscribed) {
      open = false;
      PlanUpgradeDialog.show({ message: '지금은 읽기 전용 상태예요.\nFULL ACCESS로 업그레이드하면 템플릿을 사용할 수 있어요.' });
      mixpanel.track('open_plan_upgrade_modal', { via: 'document_template' });
      return;
    }

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
        onclick={() => loadTemplate(template.entity.slug)}
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
