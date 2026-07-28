<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import dayjs from 'dayjs';
  import { userDevicePlatformLabels } from '$lib/admin-labels';
  import { AdminDataTable } from '$lib/components/admin';
  import { graphql } from '$mearie';
  import type { AdminUserSessionsTab_user$key } from '$mearie';

  type Props = {
    user$key: AdminUserSessionsTab_user$key;
  };

  let { user$key }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment AdminUserSessionsTab_user on User {
        id

        devices {
          id
          name
          platform
          lastActiveAt
          lastActiveIp
          createdAt
        }
      }
    `),
    () => user$key,
  );
</script>

<AdminDataTable
  columns={[
    { key: '$name', label: '기기', width: '28%' },
    { key: '$platform', label: '플랫폼', width: '14%' },
    { key: '$lastActive', label: '최근 활동', width: '24%' },
    { key: '$lastActiveIp', label: '최근 IP', width: '16%' },
    { key: '$createdAt', label: '등록', width: '18%' },
  ]}
  data={[...user.data.devices]}
  dataKey="id"
  emptyText="등록된 기기가 없습니다"
>
  {#snippet $name(device)}
    {device.name}
  {/snippet}

  {#snippet $platform(device)}
    <span class={css({ color: 'text.muted' })}>{userDevicePlatformLabels[device.platform]}</span>
  {/snippet}

  {#snippet $lastActive(device)}
    <span class={css({ color: 'text.muted' })}>{dayjs(device.lastActiveAt).formatAsDateTime()}</span>
  {/snippet}

  {#snippet $lastActiveIp(device)}
    <span class={css({ color: 'text.muted' })}>{device.lastActiveIp ?? '—'}</span>
  {/snippet}

  {#snippet $createdAt(device)}
    <span class={css({ color: 'text.muted' })}>{dayjs(device.createdAt).formatAsDateTime()}</span>
  {/snippet}
</AdminDataTable>
