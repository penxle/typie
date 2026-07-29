<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { attentionChipClass, chipClass, pageClass } from '$lib/styles.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();
</script>

<Helmet title="문서" trailing="타이피 평가" />

<div class={pageClass}>
  <a class={css({ fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })} href="/admin/documents">← 문서 목록</a>

  <header class={flex({ align: 'baseline', gap: '10px', marginTop: '8px', marginBottom: '20px' })}>
    <h1 class={css({ fontSize: '20px', fontWeight: 'bold' })}>{data.document.refId}</h1>
    <span class={data.document.kind === 'sampled' ? chipClass : attentionChipClass}>
      {data.document.kind === 'sampled' ? '표집' : '반입'}
    </span>
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
