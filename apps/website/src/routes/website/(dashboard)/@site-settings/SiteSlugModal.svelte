<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { TypieError } from '@typie/lib/errors';
  import { siteSchema } from '@typie/lib/validation';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal, TextInput } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import { z } from 'zod';
  import { env } from '$env/dynamic/public';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';

  type Props = {
    open: boolean;
    siteId: string;
    slug: string;
  };

  let { open = $bindable(false), siteId, slug }: Props = $props();

  const [updateSiteSlug, updateSiteSlugResult] = createMutation(
    graphql(`
      mutation DashboardLayout_SiteSettingsModal_SiteSlugModal_UpdateSiteSlug_Mutation($input: UpdateSiteSlugInput!) {
        updateSiteSlug(input: $input) {
          id
          slug
          url
        }
      }
    `),
  );

  const form = createForm({
    schema: z.object({
      slug: siteSchema.slug,
    }),
    onSubmit: async (data) => {
      if (!SubscribeModal.gate('site_settings')) return;

      await updateSiteSlug({ input: { siteId, slug: data.slug } });
      mixpanel.track('update_site_slug');
      Toast.success('스페이스 주소가 변경됐어요.');
      open = false;
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'site_slug_already_exists') {
        throw new FormError('slug', '이미 존재하는 스페이스 주소예요.');
      }

      if (error instanceof TypieError) {
        throw new FormError('slug', publicationErrorMessage(error.code));
      }
    },
    defaultValues: {
      slug,
    },
  });

  $effect(() => {
    if (open) {
      untrack(() => {
        form.reset();
        form.fields.slug = slug;
      });
    }
  });
</script>

<Modal style={css.raw({ width: '480px', padding: '24px' })} bind:open>
  <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '8px' })}>주소 변경</h2>
  <p class={css({ fontSize: '13px', color: 'text.muted', lineHeight: '[1.6]', marginBottom: '24px' })}>
    주소를 바꾸면 기존 주소로는 스페이스를 열 수 없어요.
  </p>

  <form class={flex({ direction: 'column', gap: '20px' })} onsubmit={form.handleSubmit}>
    <div class={flex({ direction: 'column', gap: '6px' })}>
      <label class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.muted' })} for="site-slug">주소</label>
      <TextInput id="site-slug" autofocus leftItemAttached bind:value={form.fields.slug}>
        {#snippet leftItem()}
          <span
            class={css({
              fontSize: '13px',
              color: 'text.muted',
              backgroundColor: 'surface.inset',
              paddingX: '12px',
              height: 'full',
              display: 'flex',
              alignItems: 'center',
            })}
          >
            {env.PUBLIC_USERSITE_HOST}/@
          </span>
        {/snippet}
      </TextInput>
      {#if form.errors.slug}
        <p class={css({ fontSize: '12px', color: 'danger.default' })}>{form.errors.slug}</p>
      {/if}
    </div>

    <div class={flex({ gap: '8px' })}>
      <Button
        style={css.raw({ flex: '1' })}
        onclick={() => {
          open = false;
        }}
        type="button"
        variant="secondary"
      >
        취소
      </Button>
      <Button style={css.raw({ flex: '1' })} loading={updateSiteSlugResult.loading} type="submit">변경</Button>
    </div>
  </form>
</Modal>
