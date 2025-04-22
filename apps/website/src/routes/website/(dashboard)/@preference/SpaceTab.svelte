<script lang="ts">
  import { fragment, graphql } from '$graphql';
  import { css } from '$styled-system/css';
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
      }
    `),
  );
</script>

<p class={css({ fontSize: '24px', fontWeight: 'bold' })}>스페이스 설정</p>

{$user.id}
