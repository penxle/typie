<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { TypieError } from '@typie/lib/errors';
  import { siteSchema } from '@typie/lib/validation';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, TextInput } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import mixpanel from 'mixpanel-browser';
  import { z } from 'zod';
  import { env } from '$env/dynamic/public';
  import { cache } from '$lib/graphql';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';

  type Props = {
    siteId: string;
    oncreated: (space: { id: string }) => void;
  };

  let { siteId, oncreated }: Props = $props();

  const [createSpace, createSpaceResult] = createMutation(
    graphql(`
      mutation DashboardLayout_NewSpaceForm_CreateSpace_Mutation($input: CreateSpaceInput!) {
        createSpace(input: $input) {
          id
          name
          slug
          url
        }
      }
    `),
  );

  const form = createForm({
    schema: z.object({
      name: z.string({ error: '스페이스 이름을 입력해주세요.' }).trim().min(1, '스페이스 이름을 입력해주세요.'),
      slug: siteSchema.slug,
    }),
    onSubmit: async (data) => {
      if (!SubscribeModal.gate('create_space')) {
        return;
      }

      const resp = await createSpace({ input: { siteId, name: data.name, slug: data.slug } });
      cache.invalidate({ __typename: 'Site', id: siteId, $field: 'spaces' });
      mixpanel.track('create_space');
      oncreated(resp.createSpace);
    },
    onError: (error) => {
      if (error instanceof TypieError && error.code === 'space_slug_already_exists') {
        throw new FormError('slug', '이미 존재하는 스페이스 주소예요.');
      }

      if (error instanceof TypieError) {
        throw new FormError('slug', publicationErrorMessage(error.code));
      }
    },
    defaultValues: {
      name: '',
      slug: '',
    },
  });
</script>

<form class={flex({ flexDirection: 'column', gap: '12px' })} onsubmit={form.handleSubmit}>
  <div class={flex({ flexDirection: 'column', gap: '6px' })}>
    <label class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })} for="new-space-name">스페이스 이름</label>
    <TextInput
      id="new-space-name"
      style={css.raw({ height: '32px', fontSize: '13px' })}
      placeholder="새 스페이스"
      bind:value={form.fields.name}
    />
    {#if form.errors.name}
      <p class={css({ fontSize: '12px', color: 'danger.default' })}>{form.errors.name}</p>
    {/if}
  </div>

  <div class={flex({ flexDirection: 'column', gap: '6px' })}>
    <label class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.muted' })} for="new-space-slug">주소</label>
    <TextInput id="new-space-slug" style={css.raw({ height: '32px', fontSize: '13px' })} rightItemAttached bind:value={form.fields.slug}>
      {#snippet rightItem()}
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
          .{env.PUBLIC_USERSITE_HOST}
        </span>
      {/snippet}
    </TextInput>
    {#if form.errors.slug}
      <p class={css({ fontSize: '12px', color: 'danger.default' })}>{form.errors.slug}</p>
    {/if}
  </div>

  <Button style={css.raw({ marginLeft: 'auto' })} loading={createSpaceResult.loading} size="sm" type="submit">스페이스 만들기</Button>
</form>
