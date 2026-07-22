<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();
</script>

<div class={css({ maxWidth: '840px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <a
    class={css({ fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })}
    href={`/admin/corpus/${data.corpusVersion}`}
  >
    ← {data.corpusVersion} 문서 목록
  </a>

  <header class={flex({ align: 'baseline', gap: '10px', marginTop: '8px', marginBottom: '20px' })}>
    <h1 class={css({ fontSize: '20px', fontWeight: 'bold' })}>{data.document.refId}</h1>
    <span class={css({ fontSize: '13px', color: 'text.faint' })}>
      {data.document.characterCount.toLocaleString()}자 · 개행 {data.document.lineBreakCount.toLocaleString()}회
    </span>
  </header>

  <article
    class={css({
      backgroundColor: 'surface.default',
      borderRadius: '12px',
      boxShadow: 'small',
      paddingX: '56px',
      paddingY: '48px',
      whiteSpace: 'pre-wrap',
      fontSize: '17px',
      lineHeight: '[1.9]',
      color: 'text.default',
      wordBreak: 'break-word',
    })}
  >
    {data.document.content}
  </article>
</div>
