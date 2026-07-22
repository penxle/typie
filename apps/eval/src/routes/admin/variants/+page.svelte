<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import VariantStatusBadge from '../VariantStatusBadge.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();
</script>

<div class={css({ maxWidth: '880px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <header class={flex({ align: 'center', justify: 'space-between', marginBottom: '20px' })}>
    <div>
      <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>후보</h1>
      <p class={css({ marginTop: '4px', fontSize: '14px', color: 'text.subtle' })}>
        프롬프트 후보 목록입니다. 계보(기반 후보)에 따라 들여써 표시됩니다.
      </p>
    </div>
    <a
      class={css({
        paddingX: '14px',
        paddingY: '9px',
        borderRadius: '8px',
        backgroundColor: 'accent.brand.default',
        color: 'text.bright',
        fontSize: '13px',
        fontWeight: 'bold',
        transition: '[background-color 0.15s ease]',
        _hover: { backgroundColor: 'accent.brand.hover' },
      })}
      href="/admin/variants/new"
    >
      + 새 후보
    </a>
  </header>

  <section
    class={css({
      backgroundColor: 'surface.default',
      borderWidth: '1px',
      borderColor: 'border.default',
      borderRadius: '12px',
      boxShadow: 'small',
      overflow: 'hidden',
    })}
  >
    {#if data.lineage.length === 0}
      <p class={css({ paddingY: '48px', textAlign: 'center', fontSize: '14px', color: 'text.faint' })}>아직 만들어진 후보가 없습니다.</p>
    {:else}
      <ul>
        {#each data.lineage as item, i (item.id)}
          <li>
            <a
              style:padding-left={`${16 + item.depth * 24}px`}
              class={flex({
                align: 'center',
                gap: '10px',
                paddingY: '12px',
                paddingRight: '16px',
                borderBottomWidth: i === data.lineage.length - 1 ? '0' : '1px',
                borderColor: 'border.subtle',
                transition: '[background-color 0.15s ease]',
                _hover: { backgroundColor: 'surface.subtle' },
              })}
              href={`/admin/variants/${item.id}`}
            >
              {#if item.depth > 0}
                <span class={css({ fontSize: '13px', color: 'text.faint', flexShrink: '0' })}>└</span>
              {/if}
              <span class={css({ fontSize: '14px', fontWeight: 'medium', flexShrink: '0' })}>{item.label}</span>
              <VariantStatusBadge status={item.status} />
              {#if item.note}
                <span
                  class={css({ fontSize: '13px', color: 'text.faint', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}
                >
                  {item.note}
                </span>
              {/if}
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>
