<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import FilePenIcon from '~icons/lucide/file-pen';
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';
  import { Button, Helmet, Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';

  const query = graphql(`
    query HomePage_Query {
      me @required {
        id

        sites {
          id

          firstEntity(type: POST) {
            id
            slug
          }
        }
      }
    }
  `);

  const createPost = graphql(`
    mutation HomePage_CreatePost_Mutation($input: CreatePostInput!) {
      createPost(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);
</script>

<Helmet title="홈" />

<div class={center({ flexDirection: 'column', gap: '20px', size: 'full', textAlign: 'center' })}>
  <Icon style={css.raw({ size: '56px', color: 'text.subtle', '& *': { strokeWidth: '[1.25px]' } })} icon={FilePenIcon} />

  <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '4px' })}>
    <h1 class={css({ fontSize: '16px', fontWeight: 'bold', color: 'text.subtle' })}>첫 포스트를 만들어보세요</h1>
    <p class={css({ fontSize: '14px', color: 'text.faint' })}>아래 버튼을 눌러 포스트를 만들 수 있어요</p>
  </div>

  <Button
    onclick={async () => {
      const resp = await createPost({
        siteId: $query.me.sites[0].id,
      });

      mixpanel.track('create_post', { via: 'empty_home' });

      await goto(`/${resp.entity.slug}`);
    }}
  >
    새 포스트 만들기
  </Button>
</div>
