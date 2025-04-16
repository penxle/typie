<script lang="ts">
  import { graphql } from '$graphql';
  import { Button } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';

  type Props = {
    postId: string;
  };

  let { postId }: Props = $props();

  let content = $state('');

  const createComment = graphql(`
    mutation UsersiteWildcardSlugPage_CreateComment_Mutation($input: CreateCommentInput!) {
      createComment(input: $input) {
        id
      }
    }
  `);
</script>

<form
  class={flex({ direction: 'column', align: 'flex-end', gap: '4px' })}
  onsubmit={async (e) => {
    e.preventDefault();
    if (content !== '') {
      await createComment({ postId, content });
      content = '';
    }
  }}
>
  <textarea
    class={css({
      borderWidth: '1px',
      borderRadius: '8px',
      borderColor: 'gray.300',
      padding: '20px',
      fontSize: '14px',
      width: 'full',
      resize: 'none',
    })}
    placeholder="응원의 댓글을 남겨주세요"
    rows="4"
    bind:value={content}
  ></textarea>

  <Button type="submit" variant="secondary">작성</Button>
</form>
