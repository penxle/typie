<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import SiteLinksEditor from './SiteLinksEditor.svelte';
  import type { DashboardLayout_SiteSettingsModal_AboutTab_site$key } from '$mearie';

  type Props = {
    site$key: DashboardLayout_SiteSettingsModal_AboutTab_site$key;
  };

  let { site$key }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_SiteSettingsModal_AboutTab_site on Site {
        id
        description

        links {
          label
          url
        }
      }
    `),
    () => site$key,
  );

  const [updateSite] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_AboutTab_UpdateSite_Mutation($input: UpdateSiteInput!) {
        updateSite(input: $input) {
          id
          description

          links {
            label
            url
          }
        }
      }
    `),
  );

  type UpdateSiteFields = Omit<Parameters<typeof updateSite>[0]['input'], 'siteId'>;

  const save = async (input: UpdateSiteFields, field: string) => {
    if (!SubscribeModal.gate('site_settings')) return false;

    try {
      await updateSite({ input: { siteId: site.data.id, ...input } });
      mixpanel.track('update_site', { field });
      Toast.success('작업실 설정이 업데이트됐어요.');
      return true;
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      return false;
    }
  };

  let description = $state(site.data.description ?? '');

  const saveDescription = async () => {
    const next = description.trim();
    if (next === (site.data.description ?? '')) return;
    if (!(await save({ description: next || null }, 'description'))) {
      description = site.data.description ?? '';
    }
  };
</script>

<div class={css({ maxWidth: '640px' })}>
  <div class={css({ marginBottom: '24px' })}>
    <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default' })}>소개</h1>
  </div>

  <div>
    <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '4px' })}>소개 문구</h2>
    <p class={css({ fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]', marginBottom: '20px' })}>
      스페이스 홈의 이름 아래에 보이는 짧은 소개예요.
    </p>

    <textarea
      class={css({
        width: 'full',
        minHeight: '72px',
        paddingX: '12px',
        paddingY: '8px',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '6px',
        fontSize: '13px',
        lineHeight: '[1.5]',
        color: 'text.default',
        backgroundColor: 'surface.default',
        resize: 'none',
        transition: 'common',
        _hover: { borderColor: 'border.emphasis' },
        _focus: { outline: 'none', borderColor: 'accent.default' },
        _placeholder: { color: 'text.hint' },
      })}
      aria-label="소개"
      onblur={saveDescription}
      placeholder="스페이스를 한두 문장으로 소개해 보세요."
      bind:value={description}></textarea>
  </div>

  <div class={css({ marginTop: '40px' })}>
    <SiteLinksEditor links={site.data.links} onsave={(links) => save({ links }, 'links')} />
  </div>
</div>
