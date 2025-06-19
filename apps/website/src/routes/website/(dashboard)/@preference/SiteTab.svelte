<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { siteSchema } from '@/validation';
  import { fragment, graphql } from '$graphql';
  import { Button, TextInput } from '$lib/components';
  import { createForm, FormError } from '$lib/form';
  import { Dialog, Toast } from '$lib/notification';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { DashboardLayout_PreferenceModal_SiteTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_SiteTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_SiteTab_user on User {
        id

        sites {
          id
          slug

          fonts {
            id
            name
            fullName
          }
        }
      }
    `),
  );

  const updateSiteSlug = graphql(`
    mutation DashboardLayout_PreferenceModal_SiteTab_UpdateSiteSlug_Mutation($input: UpdateSiteSlugInput!) {
      updateSiteSlug(input: $input) {
        id
        slug
      }
    }
  `);

  const removeSiteFont = graphql(`
    mutation DashboardLayout_PreferenceModal_SiteTab_RemoveSiteFont_Mutation($input: RemoveSiteFontInput!) {
      removeSiteFont(input: $input) {
        id

        fonts {
          id
        }
      }
    }
  `);

  const form = createForm({
    schema: z.object({
      slug: siteSchema.slug,
    }),
    onSubmit: async (data) => {
      await updateSiteSlug({ siteId: $user.sites[0].id, slug: data.slug });

      mixpanel.track('update_site_slug');
      Toast.success('사이트 주소가 변경되었습니다.');
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'site_slug_already_exists') {
        throw new FormError('slug', '이미 존재하는 사이트 주소입니다.');
      }
    },
    defaultValues: {
      slug: $user.sites[0].slug,
    },
  });
</script>

<div class={flex({ direction: 'column', gap: '32px' })}>
  <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'gray.900' })}>사이트</h1>

  <div class={flex({ direction: 'column', gap: '16px' })}>
    <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'gray.900' })}>주소</h3>

    <form class={flex({ gap: '12px' })} onsubmit={form.handleSubmit}>
      <div class={css({ width: 'full', maxWidth: '380px' })}>
        <TextInput id="slug" style={css.raw({ width: 'full' })} bind:value={form.fields.slug} />

        {#if form.errors.slug}
          <p class={css({ marginTop: '4px', color: 'red.500', fontSize: '14px' })}>{form.errors.slug}</p>
        {/if}
      </div>

      <Button style={css.raw({ flex: 'none', height: '36px' })} disabled={!form.state.isDirty} size="sm" type="submit" variant="secondary">
        변경
      </Button>
    </form>
  </div>

  <div class={flex({ direction: 'column', gap: '16px' })}>
    <h3 class={css({ fontSize: '14px', fontWeight: 'medium', color: 'gray.900' })}>폰트</h3>

    {#each $user.sites[0].fonts as { id, name, fullName } (id)}
      <div class={flex({ alignItems: 'center', gap: '8px' })}>
        <p class={css({ fontWeight: 'medium' })}>
          {name}
          {#if fullName}
            <span class={css({ fontSize: '12px', color: 'gray.500' })}>({fullName})</span>
          {/if}
        </p>

        <Button
          onclick={() => {
            Dialog.confirm({
              title: '폰트 삭제',
              message: `"${name}" 폰트를 삭제하시겠어요?`,
              action: 'danger',
              actionLabel: '삭제',
              actionHandler: async () => {
                await removeSiteFont({ siteId: $user.sites[0].id, fontId: id });
              },
            });
          }}
          size="sm"
          variant="secondary"
        >
          삭제
        </Button>
      </div>
    {:else}
      <p class={css({ fontSize: '14px', color: 'gray.500' })}>에디터에서 업로드한 폰트가 여기 나타나요</p>
    {/each}
  </div>
</div>
