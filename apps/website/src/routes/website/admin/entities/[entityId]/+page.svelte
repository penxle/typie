<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import {
    entityAvailabilityLabels,
    entityStateLabels,
    entityStateTones,
    entityTypeLabels,
    entityVisibilityLabels,
  } from '$lib/admin-labels';
  import { AdminBadge, AdminDataTable, AdminKeyValue, AdminPageHeader, AdminSection } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const entity = $derived(query.data.adminEntity);
  const title = $derived(
    entity.node.__typename === 'Document'
      ? (entity.node.title ?? '(제목 없음)')
      : entity.node.__typename === 'Folder'
        ? entity.node.name
        : '(알 수 없음)',
  );

  const path = $derived(
    entity.ancestors
      .map((ancestor) =>
        ancestor.node.__typename === 'Folder' ? ancestor.node.name : ancestor.node.__typename === 'Document' ? ancestor.node.title : '',
      )
      .filter(Boolean)
      .join(' / '),
  );
</script>

<AdminPageHeader {title}>
  {#snippet badges()}
    <AdminBadge label={entityTypeLabels[entity.type]} />
    <AdminBadge label={entityStateLabels[entity.state]} tone={entityStateTones[entity.state]} />
  {/snippet}
</AdminPageHeader>

<div class={css({ display: 'grid', gridTemplateColumns: { sm: '1fr', lg: '[1fr 1fr]' }, gap: '32px' })}>
  <div class={css({ display: 'flex', flexDirection: 'column', gap: '28px' })}>
    <AdminSection title="위치">
      <AdminKeyValue
        items={[
          { label: '사이트', value: entity.site.name },
          { label: '경로', value: path || '(최상위)' },
        ]}
      />
    </AdminSection>

    <AdminSection title="공개 설정">
      <AdminKeyValue
        items={[
          { label: '공개 범위', value: entityVisibilityLabels[entity.visibility] },
          { label: '편집 범위', value: entityAvailabilityLabels[entity.availability] },
          ...(entity.node.__typename === 'Document'
            ? [
                { label: '비밀번호', value: entity.node.password ? '설정됨' : '없음' },
                { label: '콘텐츠 등급', value: entity.node.contentRating },
                { label: '반응 허용', value: entity.node.allowReaction ? '허용' : '차단' },
                { label: '콘텐츠 보호', value: entity.node.protectContent ? '켬' : '끔' },
              ]
            : []),
        ]}
      />
    </AdminSection>
  </div>

  <div class={css({ display: 'flex', flexDirection: 'column', gap: '28px' })}>
    <AdminSection title="소유자">
      {@render owner()}
    </AdminSection>

    <AdminSection title="활동">
      <AdminKeyValue
        items={[
          ...(entity.node.__typename === 'Document' ? [{ label: '글자 수', value: comma(entity.node.characterCount) }] : []),
          { label: '생성일', value: dayjs(entity.createdAt).formatAsDateTime() },
          ...(entity.node.__typename === 'Document' ? [{ label: '수정일', value: dayjs(entity.node.updatedAt).formatAsDateTime() }] : []),
          ...(entity.state === 'ACTIVE'
            ? []
            : [{ label: '삭제일', value: entity.deletedAt ? dayjs(entity.deletedAt).formatAsDateTime() : null }]),
          ...(entity.node.__typename === 'Document' ? [{ label: '부제', value: entity.node.subtitle }] : []),
        ]}
      />
    </AdminSection>

    <AdminSection title="식별자">
      <AdminKeyValue
        items={[
          { label: 'ID', value: entity.id, mono: true },
          { label: 'slug', value: entity.slug, mono: true },
          { label: 'permalink', value: entity.permalink, mono: true },
          { label: 'URL', value: entity.url, mono: true },
        ]}
      />
    </AdminSection>
  </div>
</div>

{#snippet owner()}
  <a class={css({ fontSize: '13px', color: 'text.default', _hover: { textDecoration: 'underline' } })} href="/admin/users/{entity.user.id}">
    {entity.user.name} ({entity.user.email})
  </a>
{/snippet}

{#if entity.node.__typename === 'Document'}
  <div class={css({ marginTop: '32px' })}>
    <div class={css({ marginBottom: '10px', fontSize: '11px', fontWeight: 'semibold', letterSpacing: '[0.05em]', color: 'text.faint' })}>
      편집 이력
    </div>

    <AdminDataTable
      columns={[
        { key: '$updatedAt', label: '시각', width: '32%' },
        { key: '$characterCount', label: '글자 수', width: '20%' },
        { key: '$contributors', label: '기여자', width: '48%' },
      ]}
      data={[...entity.node.heads]}
      dataKey="id"
      emptyText="편집 이력이 없습니다"
    >
      {#snippet $updatedAt(head)}
        <span class={css({ color: 'text.muted' })}>{dayjs(head.updatedAt).formatAsDateTime()}</span>
      {/snippet}

      {#snippet $characterCount(head)}
        {comma(head.characterCount)}
      {/snippet}

      {#snippet $contributors(head)}
        {head.contributors.map((contributor) => contributor.name).join(', ')}
      {/snippet}
    </AdminDataTable>
  </div>
{/if}
