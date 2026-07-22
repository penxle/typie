<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();
</script>

<div class={css({ maxWidth: '880px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <a class={css({ fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })} href="/admin/corpus">← 코퍼스</a>

  <header class={flex({ align: 'baseline', gap: '10px', marginTop: '8px', marginBottom: '20px' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>{data.corpusVersion}</h1>
    <span class={css({ fontSize: '13px', color: 'text.faint' })}>문서 {data.documents.length.toLocaleString()}건</span>
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
