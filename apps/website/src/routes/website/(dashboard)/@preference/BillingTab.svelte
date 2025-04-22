<script lang="ts">
  import { fragment, graphql } from '$graphql';
  import { css } from '$styled-system/css';
  import type { DashboardLayout_PreferenceModal_BillingTab_user } from '$graphql';

  type Props = {
    $user: DashboardLayout_PreferenceModal_BillingTab_user;
  };

  let { $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_BillingTab_user on User {
        id
      }
    `),
  );
</script>

<p class={css({ fontSize: '24px', fontWeight: 'bold' })}>결제 설정</p>

{$user.id}
