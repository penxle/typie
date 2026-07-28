<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Select, TextInput } from '@typie/ui/components';
  import { QueryString, QueryStringNumber } from '@typie/ui/state';
  import dayjs from 'dayjs';
  import SearchIcon from '~icons/lucide/search';
  import { entityStateLabels, entityStateTones, entityTypeLabels, entityVisibilityLabels } from '$lib/admin-labels';
  import { AdminBadge, AdminDataTable, adminFilledControl, AdminFilterBar, AdminPageHeader, AdminPagination } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const searchQuery = new QueryString('search', '', { debounce: 300 });
  const typeFilter = new QueryString('type', '');
  const stateFilter = new QueryString('state', '');
  const visibilityFilter = new QueryString('visibility', '');
  const pageNumber = new QueryStringNumber('page', 1);

  const labelOf = (entity: { node: { __typename: string; title?: string | null; name?: string | null } }) =>
    entity.node.title ?? entity.node.name ?? '(제목 없음)';
</script>

<AdminPageHeader description={`총 ${query.data.adminEntities.totalCount}건`} title="엔티티" />

<AdminDataTable
  columns={[
    { key: '$title', label: '제목', width: '32%' },
    { key: '$type', label: '타입', width: '10%' },
    { key: '$state', label: '상태', width: '12%' },
    { key: '$visibility', label: '공개범위', width: '14%' },
    { key: '$user', label: '소유자', width: '16%' },
    { key: '$createdAt', label: '생성일', width: '16%' },
  ]}
  data={[...query.data.adminEntities.entities]}
  dataKey="id"
  emptyText="조건에 맞는 엔티티가 없습니다"
>
  {#snippet filters()}
    <AdminFilterBar>
      <TextInput
        style={css.raw(adminFilledControl, { maxWidth: '320px' })}
        leftIcon={SearchIcon}
        placeholder="제목, slug, permalink 또는 ID"
        size="sm"
        bind:value={
          () => searchQuery.current,
          (value) => {
            searchQuery.current = value;
            pageNumber.current = 1;
          }
        }
      />

      <Select
        style={css.raw(adminFilledControl)}
        items={[
          { label: '모든 타입', value: '' },
          { label: '문서', value: 'DOCUMENT' },
          { label: '폴더', value: 'FOLDER' },
        ]}
        onselect={() => {
          pageNumber.current = 1;
        }}
        bind:value={typeFilter.current}
      />

      <Select
        style={css.raw(adminFilledControl)}
        items={[
          { label: '모든 상태', value: '' },
          { label: '활성', value: 'ACTIVE' },
          { label: '삭제됨', value: 'DELETED' },
          { label: '완전 삭제', value: 'PURGED' },
        ]}
        onselect={() => {
          pageNumber.current = 1;
        }}
        bind:value={stateFilter.current}
      />

      <Select
        style={css.raw(adminFilledControl)}
        items={[
          { label: '모든 공개범위', value: '' },
          { label: '공개', value: 'PUBLIC' },
          { label: '링크 공개', value: 'UNLISTED' },
          { label: '비공개', value: 'PRIVATE' },
        ]}
        onselect={() => {
          pageNumber.current = 1;
        }}
        bind:value={visibilityFilter.current}
      />
    </AdminFilterBar>
  {/snippet}

  {#snippet $title(entity)}
    <a class={css({ fontWeight: 'medium', _hover: { textDecoration: 'underline' } })} href="/admin/entities/{entity.id}">
      {labelOf(entity)}
    </a>
  {/snippet}

  {#snippet $type(entity)}
    <span class={css({ color: 'text.muted' })}>{entityTypeLabels[entity.type]}</span>
  {/snippet}

  {#snippet $state(entity)}
    <AdminBadge label={entityStateLabels[entity.state]} tone={entityStateTones[entity.state]} />
  {/snippet}

  {#snippet $visibility(entity)}
    <span class={css({ color: 'text.muted' })}>{entityVisibilityLabels[entity.visibility]}</span>
  {/snippet}

  {#snippet $user(entity)}
    <a class={css({ _hover: { textDecoration: 'underline' } })} href="/admin/users/{entity.user.id}">{entity.user.name}</a>
  {/snippet}

  {#snippet $createdAt(entity)}
    <span class={css({ color: 'text.muted' })}>{dayjs(entity.createdAt).formatAsDateTime()}</span>
  {/snippet}

  {#snippet footer()}
    <AdminPagination totalCount={query.data.adminEntities.totalCount} bind:pageNumber={pageNumber.current} />
  {/snippet}
</AdminDataTable>
