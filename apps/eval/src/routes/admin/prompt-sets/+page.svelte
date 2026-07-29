<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { untrack } from 'svelte';
  import { enhance } from '$app/forms';
  import {
    emptyClass,
    formInputClass,
    formLabelClass,
    formNoticeClass,
    formSubmitClass,
    pageClass,
    pageDescClass,
    pageTitleClass,
    panelClass,
  } from '$lib/styles.ts';
  import PromptSetStatusBadge from '../PromptSetStatusBadge.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData; form: { message?: string } | null };
  const { data, form }: Props = $props();

  const active = $derived(data.generations.filter((g) => g.status === 'active'));
  let generationId = $state(untrack(() => data.generations.find((g) => g.status === 'active')?.id ?? ''));
</script>

<Helmet title="프롬프트 묶음" trailing="타이피 평가" />

<div class={pageClass}>
  <header class={css({ marginBottom: '20px' })}>
    <h1 class={pageTitleClass}>프롬프트 묶음</h1>
    <p class={pageDescClass}>세대를 고르면 그 세대의 단계로 편집 폼이 만들어집니다.</p>
  </header>

  <section
    class={css({
      marginBottom: '24px',
      backgroundColor: 'surface.default',
      borderWidth: '1px',
      borderColor: 'border.default',
      borderRadius: '12px',
      padding: '20px',
      boxShadow: 'small',
    })}
  >
    <h2 class={css({ fontSize: '14px', fontWeight: 'bold', marginBottom: '12px' })}>새 묶음</h2>
    <form action="?/create" method="post" use:enhance>
      <div class={grid({ columns: 3, gap: '12px', alignItems: 'end' })}>
        <div>
          <label class={formLabelClass} for="new-set-generation">세대</label>
          <select id="new-set-generation" name="generationId" class={formInputClass} bind:value={generationId}>
            {#each active as generation (generation.id)}
              <option value={generation.id}>{generation.label} · {generation.phases.length}단계</option>
            {/each}
          </select>
        </div>
        <div>
          <label class={formLabelClass} for="new-set-label">라벨</label>
          <input id="new-set-label" name="label" class={formInputClass} placeholder="예: v4 초안" required type="text" />
        </div>
        <button class={formSubmitClass} disabled={!generationId} type="submit">+ 새 묶음</button>
      </div>
      <p class={[formNoticeClass, css({ color: 'text.danger' })]}>{form?.message ?? ''}</p>
    </form>
  </section>

  <section class={panelClass}>
    {#if data.sets.length === 0}
      <p class={emptyClass}>아직 만들어진 묶음이 없습니다.</p>
    {:else}
      <ul>
        {#each data.sets as set, i (set.id)}
          <li>
            <a
              class={flex({
                align: 'center',
                gap: '10px',
                paddingX: '16px',
                paddingY: '12px',
                borderBottomWidth: i === data.sets.length - 1 ? '0' : '1px',
                borderColor: 'border.subtle',
                transition: '[background-color 0.15s ease]',
                _hover: { backgroundColor: 'surface.subtle' },
              })}
              href={`/admin/prompt-sets/${set.id}`}
            >
              <span class={css({ fontSize: '14px', fontWeight: 'medium', flexShrink: '0' })}>{set.label}</span>
              <PromptSetStatusBadge status={set.status} />
              <span class={css({ fontSize: '13px', color: 'text.faint', flexShrink: '0' })}>{set.generationLabel}</span>
              {#if set.note}
                <span
                  class={css({ fontSize: '13px', color: 'text.faint', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}
                >
                  {set.note}
                </span>
              {/if}
              <span class={css({ marginLeft: 'auto', fontSize: '12px', color: 'text.faint', flexShrink: '0' })}>
                {new Date(set.createdAt).toLocaleDateString('ko')}
              </span>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>
