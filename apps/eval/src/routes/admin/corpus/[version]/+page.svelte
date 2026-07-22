<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { GENRES } from '../../../../../flows/src/genres.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const GENRE_NAMES = new Map<string, string>(GENRES.map((genre) => [genre.key, genre.name]));

  const genreLabel = (key: string): string => (key === 'unclassified' ? '장르 정보 없음' : (GENRE_NAMES.get(key) ?? key));

  const orderedEntries = (dist: Record<string, number>): [string, number][] => {
    const distKeys = new Set(Object.keys(dist));
    const known = GENRES.map((genre) => genre.key).filter((key) => distKeys.has(key));
    const rest = [...distKeys].filter((key) => !GENRE_NAMES.has(key));
    return [...known, ...rest].map((key) => [key, dist[key]]);
  };

  const percentage = (count: number, total: number): number => (total > 0 ? Math.round((count / total) * 100) : 0);

  const frozenEntries = $derived(orderedEntries(data.frozen));
  const frozenTotal = $derived(frozenEntries.reduce((sum, [, count]) => sum + count, 0));
  const poolEntries = $derived(data.pool ? orderedEntries(data.pool) : null);
  const poolTotal = $derived(poolEntries?.reduce((sum, [, count]) => sum + count, 0) ?? 0);
</script>

<Helmet title={`코퍼스 ${data.corpusVersion}`} trailing="타이피 평가" />

<div class={css({ maxWidth: '880px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <a class={css({ fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })} href="/admin/corpus">← 코퍼스</a>

  <header class={flex({ align: 'baseline', gap: '10px', marginTop: '8px', marginBottom: '20px' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>{data.corpusVersion}</h1>
    <span class={css({ fontSize: '13px', color: 'text.faint' })}>문서 {data.documents.length.toLocaleString()}건</span>
  </header>

  <section class={grid({ columns: 2, gap: '16px', marginBottom: '24px' })}>
    <div
      class={css({
        backgroundColor: 'surface.default',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '12px',
        boxShadow: 'small',
        padding: '16px',
      })}
    >
      <h2 class={css({ fontSize: '13px', fontWeight: 'bold', marginBottom: '10px' })}>후보 풀 분포</h2>
      {#if poolEntries}
        <div class={flex({ direction: 'column', gap: '4px' })}>
          {#each poolEntries as [key, count] (key)}
            <div class={flex({ justify: 'space-between', fontSize: '13px' })}>
              <span>{genreLabel(key)}</span>
              <span class={css({ color: 'text.subtle' })}>{count.toLocaleString()}건 ({percentage(count, poolTotal)}%)</span>
            </div>
          {/each}
        </div>
      {:else}
        <p class={css({ fontSize: '13px', color: 'text.faint' })}>후보 분포 정보 없음</p>
      {/if}
    </div>

    <div
      class={css({
        backgroundColor: 'surface.default',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '12px',
        boxShadow: 'small',
        padding: '16px',
      })}
    >
      <h2 class={css({ fontSize: '13px', fontWeight: 'bold', marginBottom: '10px' })}>동결 분포</h2>
      <div class={flex({ direction: 'column', gap: '4px' })}>
        {#each frozenEntries as [key, count] (key)}
          <div
            class={flex({
              justify: 'space-between',
              fontSize: '13px',
              color: key === 'unclassified' ? 'text.faint' : 'text.default',
            })}
          >
            <span>{genreLabel(key)}</span>
            <span class={css({ color: 'text.subtle' })}>{count.toLocaleString()}건 ({percentage(count, frozenTotal)}%)</span>
          </div>
        {/each}
      </div>
    </div>
  </section>

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
    <table class={css({ width: 'full', fontSize: '13px', '& td, & th': { paddingX: '16px', paddingY: '10px', textAlign: 'left' } })}>
      <thead>
        <tr
          class={css({
            '& th': { color: 'text.faint', fontWeight: 'medium', borderBottomWidth: '1px', borderColor: 'border.default' },
          })}
        >
          <th>refId</th>
          <th>글자수</th>
          <th>개행 수</th>
        </tr>
      </thead>
      <tbody>
        {#each data.documents as document (document.id)}
          <tr class={css({ '& td': { borderBottomWidth: '1px', borderColor: 'border.subtle' } })}>
            <td>
              <a
                class={css({ fontWeight: 'bold', color: 'text.link', transition: '[color 0.15s ease]', _hover: { color: 'text.brand' } })}
                href={`/admin/corpus/${data.corpusVersion}/${document.id}`}
              >
                {document.refId}
              </a>
            </td>
            <td>{document.characterCount.toLocaleString()}</td>
            <td>{document.lineBreakCount.toLocaleString()}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>
</div>
