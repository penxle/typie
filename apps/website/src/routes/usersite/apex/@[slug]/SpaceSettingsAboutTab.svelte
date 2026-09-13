<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { cache } from '$lib/graphql';
  import { publicationErrorCode } from '$lib/publication/error';
  import { publicationErrorMessage } from '$lib/publication/publish-form';
  import { graphql } from '$mearie';
  import SpaceLinksEditor from './SpaceLinksEditor.svelte';
  import type { UsersiteSpace_SpaceSettingsAboutTab_space$key } from '$mearie';

  type Props = {
    space$key: UsersiteSpace_SpaceSettingsAboutTab_space$key;
  };

  let { space$key }: Props = $props();

  const space = createFragment(
    graphql(`
      fragment UsersiteSpace_SpaceSettingsAboutTab_space on Space {
        id
        description

        links {
          label
          url
        }
      }
    `),
    () => space$key,
  );

  const [updateSpace] = createMutation(
    graphql(`
      mutation UsersiteSpace_SpaceSettingsAboutTab_UpdateSpace_Mutation($input: UpdateSpaceInput!) {
        updateSpace(input: $input) {
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

  type UpdateSpaceFields = Omit<Parameters<typeof updateSpace>[0]['input'], 'spaceId'>;

  const save = async (input: UpdateSpaceFields, field: string) => {
    try {
      await updateSpace({ input: { spaceId: space.data.id, ...input } });
      cache.invalidate({ __typename: 'Query', $field: 'spaceView' });
      mixpanel.track('update_space', { field, via: 'space_page' });
      Toast.success('스페이스 설정이 업데이트됐어요.');
      return true;
    } catch (err) {
      Toast.error(publicationErrorMessage(publicationErrorCode(err)));
      return false;
    }
  };

  let description = $state(space.data.description ?? '');

  const saveDescription = async () => {
    const next = description.trim();
    if (next === (space.data.description ?? '')) return;
    if (!(await save({ description: next || null }, 'description'))) {
      description = space.data.description ?? '';
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
    <SpaceLinksEditor links={space.data.links} onsave={(links) => save({ links }, 'links')} />
  </div>
</div>
