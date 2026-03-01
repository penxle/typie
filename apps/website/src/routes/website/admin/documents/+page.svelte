<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { QueryString, QueryStringNumber } from '@typie/ui/state';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import SearchIcon from '~icons/lucide/search';
  import { AdminIcon, AdminPagination, AdminTable } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const searchQuery = new QueryString('search', '', { debounce: 300 });
  const pageNumber = new QueryStringNumber('page', 1);
</script>

<div class={flex({ flexDirection: 'column', gap: '24px', color: 'amber.500' })}>
  <div>
    <h2 class={css({ fontSize: '18px', color: 'amber.500' })}>DOCUMENT MANAGEMENT</h2>
    <p class={css({ marginTop: '8px', fontSize: '13px', color: 'amber.400' })}>
      TOTAL DOCUMENTS: {query.data.adminDocuments.totalCount}
    </p>
  </div>

  <div
    class={css({
      borderWidth: '2px',
      borderColor: 'amber.500',
      backgroundColor: 'gray.900',
    })}
  >
    <div class={css({ padding: '20px', borderBottomWidth: '2px', borderColor: 'amber.500' })}>
      <div class={css({ position: 'relative', maxWidth: '480px' })}>
        <AdminIcon
          style={css.raw({
            position: 'absolute',
            left: '12px',
            top: '[50%]',
            transform: 'translateY(-50%)',
            color: 'amber.500',
          })}
          icon={SearchIcon}
          size={16}
        />
        <input
          class={css({
            width: 'full',
            paddingLeft: '36px',
            paddingRight: '12px',
            paddingY: '8px',
            borderWidth: '2px',
            borderColor: 'amber.500',
            backgroundColor: 'gray.800',
            color: 'amber.500',
            fontSize: '13px',
            outline: 'none',
            caretColor: 'amber.500',
            _placeholder: {
              color: 'amber.400',
              opacity: '[0.5]',
            },
            _focus: {
              borderColor: 'amber.400',
            },
          })}
          placeholder="SEARCH ID, SLUG, PERMALINK OR TITLE..."
          type="text"
          bind:value={
            () => searchQuery.current,
            (value) => {
              searchQuery.current = value;
              pageNumber.current = 1;
            }
          }
        />
      </div>
    </div>

    <AdminTable
      columns={[
        { key: '$document', label: 'DOCUMENT', width: '25%' },
        { key: '$id', label: 'ID', width: '10%' },
        { key: '$user', label: 'USER', width: '15%' },
        { key: '$path', label: 'PATH', width: '20%' },
        { key: '$characters', label: 'CHARACTERS', width: '10%' },
        { key: '$state', label: 'STATE', width: '10%' },
        { key: '$updatedAt', label: 'UPDATED', width: '10%' },
      ]}
      data={[...query.data.adminDocuments.documents]}
      dataKey="id"
    >
      {#snippet $document(doc)}
        <div class={flex({ alignItems: 'center', gap: '16px' })}>
          <div
            class={css({
              borderRadius: '8px',
              size: '40px',
              backgroundColor: 'amber.500',
              overflow: 'hidden',
            })}
          >
            {#if doc.thumbnail?.url}
              <img alt={doc.title} src={doc.thumbnail.url} />
            {/if}
          </div>
          <div>
            <a
              class={css({
                fontSize: '13px',
                color: 'amber.500',
                _hover: { textDecoration: 'underline' },
              })}
              href="/admin/documents/{doc.id}"
            >
              {doc.title}
            </a>
            {#if doc.subtitle}
              <div class={css({ fontSize: '11px', color: 'amber.400' })}>
                {doc.subtitle}
              </div>
            {/if}
          </div>
        </div>
      {/snippet}

      {#snippet $id(doc)}
        <div class={flex({ flexDirection: 'column', gap: '2px' })}>
          <span class={css({ fontSize: '11px', color: 'gray.400' })}>
            {doc.id}
          </span>
          <span class={css({ fontSize: '11px', color: 'gray.400' })}>
            {doc.entity.id}
          </span>
        </div>
      {/snippet}

      {#snippet $user(doc)}
        {#if doc.entity?.user}
          <div class={flex({ alignItems: 'center', gap: '8px' })}>
            <div
              class={css({
                size: '24px',
                borderRadius: 'full',
                backgroundColor: 'amber.500',
                overflow: 'hidden',
                flexShrink: '0',
              })}
            >
              {#if doc.entity.user.avatar?.url}
                <img alt={doc.entity.user.name} src={doc.entity.user.avatar.url} />
              {/if}
            </div>
            <div>
              <a
                class={css({
                  fontSize: '12px',
                  color: 'amber.500',
                  _hover: { textDecoration: 'underline' },
                })}
                href="/admin/users/{doc.entity.user.id}"
              >
                {doc.entity.user.name}
              </a>
            </div>
          </div>
        {:else}
          <span class={css({ fontSize: '12px', color: 'gray.400' })}>-</span>
        {/if}
      {/snippet}

      {#snippet $path(doc)}
        <div class={flex({ fontSize: '12px', color: 'amber.400', alignItems: 'center', gap: '4px' })}>
          {#if doc.entity.ancestors.length > 0}
            {#each doc.entity.ancestors as ancestor, i (ancestor.id)}
              <span>
                {ancestor.node.__typename === 'Folder'
                  ? ancestor.node.name
                  : ancestor.node.__typename === 'Document'
                    ? ancestor.node.title
                    : ''}
              </span>
              {#if i < doc.entity.ancestors.length - 1}
                <AdminIcon icon={ChevronRightIcon} size={12} />
              {/if}
            {/each}
          {:else}
            <span class={css({ color: 'gray.500' })}>-</span>
          {/if}
        </div>
      {/snippet}

      {#snippet $characters(doc)}
        <span class={css({ fontSize: '12px', color: 'amber.400' })}>
          {comma(doc.characterCount)} CHARS
        </span>
      {/snippet}

      {#snippet $state(doc)}
        <span
          class={css({
            fontSize: '12px',
            color: doc.entity.state === 'ACTIVE' ? 'green.400' : 'red.400',
          })}
        >
          {doc.entity.state}
        </span>
      {/snippet}

      {#snippet $updatedAt(doc)}
        <span class={css({ fontSize: '12px', color: 'amber.400' })}>
          {dayjs(doc.updatedAt).formatAsDateTime()}
        </span>
      {/snippet}
    </AdminTable>

    <AdminPagination totalCount={query.data.adminDocuments.totalCount} bind:pageNumber={pageNumber.current} />
  </div>
</div>
