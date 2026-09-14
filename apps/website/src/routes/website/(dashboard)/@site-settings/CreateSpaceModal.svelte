<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { TypieError } from '@typie/lib/errors';
  import { siteSchema } from '@typie/lib/validation';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal, TextInput } from '@typie/ui/components';
  import { createForm, FormError } from '@typie/ui/form';
  import mixpanel from 'mixpanel-browser';
  import { untrack } from 'svelte';
  import { z } from 'zod';
  import { env } from '$env/dynamic/public';
  import { cache } from '$lib/graphql';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';

  type Props = {
    open: boolean;
    siteId: string;
  };

  let { open = $bindable(false), siteId }: Props = $props();

  const [createSpace, createSpaceResult] = createMutation(
    graphql(`
      mutation DashboardLayout_CreateSpaceModal_CreateSpace_Mutation($input: CreateSpaceInput!) {
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

      await createSpace({ input: { siteId, name: data.name, slug: data.slug } });
      cache.invalidate({ __typename: 'Site', id: siteId, $field: 'spaces' });

      mixpanel.track('create_space', { via: 'site_settings' });
      open = false;
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

  $effect(() => {
    if (open) {
      untrack(() => form.reset());
    }
  });
</script>

<Modal
  style={css.raw({
    padding: '24px',
    maxWidth: '400px',
  })}
  bind:open
>
  <form class={flex({ flexDirection: 'column', gap: '24px' })} onsubmit={form.handleSubmit}>
    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <div class={css({ fontSize: '15px', fontWeight: 'bold', letterSpacing: '-0.01em', color: 'text.default' })}>새 스페이스 생성</div>
      <div class={css({ fontSize: '13px', color: 'text.muted', wordBreak: 'keep-all' })}>
        스페이스는 발행한 글이 모이는 공개 공간이에요.
        <br />
        이름과 주소는 나중에 바꿀 수 있어요.
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '6px' })}>
      <label class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })} for="create-space-name">스페이스 이름</label>
      <TextInput id="create-space-name" autofocus placeholder="새 스페이스" size="md" bind:value={form.fields.name} />
      {#if form.errors.name}
        <p class={css({ fontSize: '12px', color: 'danger.default' })}>{form.errors.name}</p>
      {/if}
    </div>

    <div class={flex({ flexDirection: 'column', gap: '6px' })}>
      <label class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })} for="create-space-slug">주소</label>
      <TextInput id="create-space-slug" leftItemAttached size="md" bind:value={form.fields.slug}>
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

    <div class={flex({ justifyContent: 'flex-end', gap: '10px' })}>
      <Button
        style={css.raw({ paddingX: '16px' })}
        onclick={() => {
          open = false;
        }}
        type="button"
        variant="secondary"
      >
        취소
      </Button>
      <Button style={css.raw({ paddingX: '16px' })} loading={createSpaceResult.loading} type="submit">생성</Button>
    </div>
  </form>
</Modal>
