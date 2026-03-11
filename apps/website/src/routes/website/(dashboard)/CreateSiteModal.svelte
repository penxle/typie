<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal, TextInput } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import mixpanel from 'mixpanel-browser';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';

  type Props = {
    open: boolean;
    userId: string;
  };

  let { open = $bindable(false), userId }: Props = $props();

  const app = getAppContext();

  const [createSiteMutation, createSiteMutationResult] = createMutation(
    graphql(`
      mutation DashboardLayout_CreateSiteModal_CreateSite_Mutation($input: CreateSiteInput!) {
        createSite(input: $input) {
          id
          name

          logo {
            id
            ...Img_image
          }

          ...DashboardLayout_EntityTree_site
        }
      }
    `),
  );

  let name = $state('');

  const handleSubmit = async () => {
    const resp = await createSiteMutation({
      input: { name: name.trim() || '새 스페이스' },
    });

    app.state.nextCurrentSiteId = resp.createSite.id;
    cache.invalidate({ __typename: 'User', id: userId, $field: 'sites' });

    mixpanel.track('create_site', { via: 'sidebar' });
    open = false;
    name = '';
  };

  $effect(() => {
    if (open) {
      name = '';
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
  <form
    class={flex({ flexDirection: 'column', gap: '24px' })}
    onsubmit={(e) => {
      e.preventDefault();
      handleSubmit();
    }}
  >
    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <div class={css({ fontSize: '15px', fontWeight: 'bold', letterSpacing: '-0.01em', color: 'text.default' })}>새 스페이스 생성</div>
      <div class={css({ fontSize: '13px', color: 'text.muted', wordBreak: 'keep-all' })}>
        스페이스는 독립된 글쓰기 공간이에요.
        <br />
        주제나 목적에 따라 글을 나누어 관리해보세요.
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '6px' })}>
      <label class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })} for="create-site-name">스페이스 이름</label>
      <TextInput id="create-site-name" autofocus placeholder="새 스페이스" size="md" bind:value={name} />
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
      <Button style={css.raw({ paddingX: '16px' })} loading={createSiteMutationResult.loading} type="submit">생성</Button>
    </div>
  </form>
</Modal>
