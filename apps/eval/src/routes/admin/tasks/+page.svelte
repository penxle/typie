<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const STAGE_LABELS: Record<string, string> = { screening: '스크리닝', confirmation: '확정' };

  const cardClass = css({
    backgroundColor: 'surface.default',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '12px',
    padding: '20px',
    boxShadow: 'small',
    marginBottom: '16px',
  });

  const tableClass = css({
    width: 'full',
    fontSize: '13px',
    '& th': { textAlign: 'left', paddingY: '6px', paddingRight: '12px', color: 'text.faint', fontWeight: 'medium' },
    '& td': { paddingY: '6px', paddingRight: '12px', borderTopWidth: '1px', borderColor: 'border.subtle' },
  });
</script>

<Helmet title="태스크" trailing="타이피 평가 어드민" />

<div class={css({ maxWidth: '1080px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <header class={css({ marginBottom: '20px' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>태스크</h1>
    <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>
      평가자에게 보이는 화면 그대로 미리봅니다. 미리보기에서는 아무것도 저장되지 않습니다.
    </p>
  </header>

  {#if data.rounds.length === 0}
    <p class={css({ fontSize: '13px', color: 'text.faint' })}>라운드가 없습니다.</p>
  {/if}

  {#each data.rounds as round (round.id)}
    <section class={cardClass}>
      <h2 class={css({ fontSize: '14px', fontWeight: 'bold', marginBottom: '10px' })}>
        {STAGE_LABELS[round.stage] ?? round.stage}
        <span class={css({ marginLeft: '6px', fontWeight: 'normal', color: 'text.faint', fontSize: '12px' })}>
          {round.id} · 태스크 {round.tasks.length}개
        </span>
      </h2>
      <table class={tableClass}>
        <thead>
          <tr>
            <th>태스크</th>
            <th>코퍼스</th>
            <th>글자수</th>
            <th>판정</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each round.tasks as task (task.id)}
            <tr>
              <td class={css({ fontVariantNumeric: 'tabular-nums' })}>{task.id.slice(0, 8)}</td>
              <td>{task.corpusVersion}</td>
              <td class={css({ fontVariantNumeric: 'tabular-nums' })}>{task.characterCount.toLocaleString()}</td>
              <td class={css({ fontVariantNumeric: 'tabular-nums' })}>{task.confirmed} / {task.required}</td>
              <td>
                <a
                  class={css({ fontSize: '12px', color: 'accent.brand.default', _hover: { textDecoration: 'underline' } })}
                  href={`/admin/tasks/${task.id}`}
                >
                  미리보기 →
                </a>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>
  {/each}
</div>
