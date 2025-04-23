<script lang="ts">
  import { z } from 'zod';
  import { TypieError } from '@/errors';
  import { siteSchema } from '@/validation';
  import { fragment, graphql } from '$graphql';
  import { Button, TextInput } from '$lib/components';
  import { createForm, FormError } from '$lib/form';
  import { Toast } from '$lib/notification';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { DashboardLayout_PreferenceModal_SpaceTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_SpaceTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_SpaceTab_user on User {
        id

        sites {
          id
          slug
        }
      }
    `),
  );

  const updateSiteSlug = graphql(`
    mutation DashboardLayout_PreferenceModal_SpaceTab_UpdateSiteSlug_Mutation($input: UpdateSiteSlugInput!) {
      updateSiteSlug(input: $input) {
        id
        slug
      }
    }
  `);

  const form = createForm({
    schema: z.object({
      slug: siteSchema.slug,
    }),
    onSubmit: async (data) => {
      await updateSiteSlug({ siteId: $user.sites[0].id, slug: data.slug });
      Toast.success('스페이스 주소가 변경되었습니다.');
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'site_slug_already_exists') {
        throw new FormError('slug', '이미 존재하는 스페이스 주소입니다.');
      }
    },
    defaultValues: {
      slug: $user.sites[0].slug,
    },
  });
</script>

<div class={flex({ direction: 'column', gap: '24px' })}>
  <p class={css({ fontSize: '20px', fontWeight: 'bold' })}>스페이스 설정</p>

  <div class={flex({ direction: 'column', gap: '8px' })}>
    <p class={css({ fontWeight: 'medium' })}>주소</p>

    <form class={flex({ gap: '12px' })} onsubmit={form.handleSubmit}>
      <div class={css({ width: 'full', maxWidth: '380px' })}>
        <TextInput id="slug" style={css.raw({ width: 'full' })} bind:value={form.fields.slug} />

        {#if form.errors.slug}
          <p class={css({ marginTop: '4px', color: 'red.500', fontSize: '14px' })}>{form.errors.slug}</p>
        {/if}
      </div>

      <Button style={css.raw({ flex: 'none', height: '38px' })} disabled={!form.state.isDirty} type="submit">주소 변경</Button>
    </form>
  </div>
</div>
