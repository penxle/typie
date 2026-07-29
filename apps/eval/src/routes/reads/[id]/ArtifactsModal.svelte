<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, Modal } from '@typie/ui/components';
  import IconX from '~icons/lucide/x';
  import type { EditorialPlan, Research } from '../../../../flows/src/editorial-types.ts';

  type Props = { open: boolean; research: Research; plan: EditorialPlan };
  let { open = $bindable(), research, plan }: Props = $props();

  let activeTab = $state<'research' | 'plan'>('research');

  const tabClass = (selected: boolean) =>
    css({
      paddingY: '10px',
      borderBottomWidth: '2px',
      borderColor: selected ? 'accent.brand.default' : 'transparent',
      color: selected ? 'text.default' : 'text.faint',
      fontSize: '13px',
      fontWeight: selected ? 'bold' : 'normal',
      cursor: 'pointer',
      _hover: { color: 'text.default' },
    });

  const sectionClass = css({ marginBottom: '20px' });
  const sectionTitleClass = css({ fontSize: '13px', fontWeight: 'bold', marginBottom: '8px' });
  const cardClass = css({
    backgroundColor: 'surface.subtle',
    borderRadius: '8px',
    padding: '12px',
    marginBottom: '8px',
    _last: { marginBottom: '0' },
  });
  const fieldLabelClass = css({ flexShrink: '0', width: '72px', fontSize: '12px', color: 'text.faint' });
  const fieldBodyClass = css({ fontSize: '13px', color: 'text.default', lineHeight: '[1.6]', whiteSpace: 'pre-wrap' });
  const quoteClass = css({
    borderLeftWidth: '2px',
    borderColor: 'border.strong',
    paddingLeft: '8px',
    fontSize: '12px',
    color: 'text.subtle',
    lineHeight: '[1.5]',
    whiteSpace: 'pre-wrap',
    marginTop: '4px',
  });
  const badgeClass = css({
    display: 'inline-block',
    paddingX: '6px',
    paddingY: '1px',
    borderRadius: 'full',
    backgroundColor: 'surface.muted',
    fontSize: '11px',
    fontFamily: 'mono',
    color: 'text.subtle',
  });
</script>

{#snippet field(label: string, body: string)}
  {#if body}
    <div class={flex({ gap: '8px', marginBottom: '6px', _last: { marginBottom: '0' } })}>
      <span class={fieldLabelClass}>{label}</span>
      <span class={fieldBodyClass}>{body}</span>
    </div>
  {/if}
{/snippet}

{#snippet quotes(items: string[])}
  {#each items as quote, i (i)}
    <p class={quoteClass}>{quote}</p>
  {/each}
{/snippet}

<!-- Modal 상자 자체가 스크롤 컨테이너라 탭 헤더가 함께 밀려 올라간다 — 상자 스크롤을 끄고
  본문만 스크롤시켜 탭을 고정한다. -->
<Modal style={css.raw({ maxHeight: '[85dvh]', overflowY: 'hidden' })} bind:open>
  <div
    class={flex({
      align: 'center',
      gap: '20px',
      paddingX: '20px',
      borderBottomWidth: '1px',
      borderColor: 'border.default',
      flexShrink: '0',
    })}
  >
    <button class={tabClass(activeTab === 'research')} onclick={() => (activeTab = 'research')} type="button">리서치</button>
    <button class={tabClass(activeTab === 'plan')} onclick={() => (activeTab = 'plan')} type="button">비평 계획</button>
    <button
      class={css({ marginLeft: 'auto', color: 'text.faint', cursor: 'pointer', _hover: { color: 'text.default' } })}
      aria-label="닫기"
      onclick={() => (open = false)}
      type="button"
    >
      <Icon icon={IconX} size={16} />
    </button>
  </div>

  <div class={css({ overflowY: 'auto', padding: '20px' })}>
    {#if activeTab === 'research'}
      <section class={sectionClass}>
        <h3 class={sectionTitleClass}>글의 성격</h3>
        <div class={cardClass}>
          {@render field('형식', research.nature.form)}
          <div class={flex({ gap: '8px', marginBottom: '6px' })}>
            <span class={fieldLabelClass}>완성도</span>
            <span class={fieldBodyClass}>
              <span class={badgeClass}>{research.nature.completeness.level}</span>
              {research.nature.completeness.note}
            </span>
          </div>
          {@render field('검토 한계', research.nature.feedbackFit)}
        </div>
      </section>

      <section class={sectionClass}>
        <h3 class={sectionTitleClass}>시점·문체</h3>
        <div class={cardClass}>
          {@render field('시점', research.voice.pov)}
        </div>
        {#each research.voice.conventions as convention, i (i)}
          <div class={cardClass}>
            <p class={fieldBodyClass}>{convention.pattern}</p>
            {@render quotes(convention.evidence)}
          </div>
        {/each}
      </section>

      {#if research.names.length > 0}
        <section class={sectionClass}>
          <h3 class={sectionTitleClass}>인물·용어</h3>
          <div class={cardClass}>
            {#each research.names as entry, i (i)}
              <div class={css({ marginBottom: '6px', _last: { marginBottom: '0' } })}>
                <span class={css({ fontSize: '13px', fontWeight: 'medium' })}>{entry.name}</span>
                {#if entry.aliases.length > 0}
                  <span class={css({ fontSize: '12px', color: 'text.faint' })}>({entry.aliases.join(', ')})</span>
                {/if}
                {#if entry.note}
                  <span class={css({ fontSize: '12px', color: 'text.subtle' })}>— {entry.note}</span>
                {/if}
              </div>
            {/each}
          </div>
        </section>
      {/if}

      <section class={sectionClass}>
        <h3 class={sectionTitleClass}>독자 전제</h3>
        <div class={cardClass}>
          <div class={flex({ gap: '8px', marginBottom: '6px' })}>
            <span class={fieldLabelClass}>원작</span>
            <span class={fieldBodyClass}>
              <span class={badgeClass}>{research.premise.sourceWork.status}</span>
              {[research.premise.sourceWork.name, research.premise.sourceWork.brief].filter(Boolean).join(' — ')}
            </span>
          </div>
          {@render field('장르 문법', research.premise.genreConventions)}
          {@render field('연작 맥락', research.premise.seriesContext)}
        </div>
      </section>

      {#if research.boundaries.length > 0}
        <section class={sectionClass}>
          <h3 class={sectionTitleClass}>분석 제외 구간</h3>
          {#each research.boundaries as boundary, i (i)}
            <div class={cardClass}>
              <p class={fieldBodyClass}>{boundary.reason}</p>
              {@render quotes([`${boundary.startQuote} … ${boundary.endQuote}`])}
            </div>
          {/each}
        </section>
      {/if}

      {#if research.unverified.length > 0}
        <section class={sectionClass}>
          <h3 class={sectionTitleClass}>미확정 전제</h3>
          <div class={cardClass}>
            <ul class={css({ listStyleType: 'disc', paddingLeft: '16px' })}>
              {#each research.unverified as item, i (i)}
                <li class={fieldBodyClass}>{item}</li>
              {/each}
            </ul>
          </div>
        </section>
      {/if}
    {:else}
      <section class={sectionClass}>
        <h3 class={sectionTitleClass}>의도</h3>
        <div class={cardClass}>
          <p class={fieldBodyClass}>{plan.intent}</p>
        </div>
      </section>

      {#if plan.protected.length > 0}
        <section class={sectionClass}>
          <h3 class={sectionTitleClass}>보호 기법 ({plan.protected.length})</h3>
          {#each plan.protected as item, i (i)}
            <div class={cardClass}>
              <p class={css({ fontSize: '13px', fontWeight: 'medium', marginBottom: '4px' })}>{item.technique}</p>
              <p class={fieldBodyClass}>{item.rationale}</p>
              {@render quotes(item.evidence)}
            </div>
          {/each}
        </section>
      {/if}

      <section class={sectionClass}>
        <h3 class={sectionTitleClass}>검토 축 ({plan.axes.length})</h3>
        {#each plan.axes as axis, i (i)}
          <div class={cardClass}>
            <p class={css({ fontSize: '13px', fontWeight: 'medium', marginBottom: '6px' })}>{axis.label}</p>
            {@render field('검토 지시', axis.inquiry)}
            {@render field('위험', axis.risk)}
            <div class={flex({ gap: '8px', marginBottom: '6px' })}>
              <span class={fieldLabelClass}>규약 대조</span>
              <span class={fieldBodyClass}>
                <span class={badgeClass}>{axis.conventionsBasis}</span>
                {axis.conventionsCheck}
              </span>
            </div>
            {@render quotes(axis.evidence)}
          </div>
        {/each}
      </section>

      {#if plan.verifications.length > 0}
        <section class={sectionClass}>
          <h3 class={sectionTitleClass}>도구 검증 ({plan.verifications.length})</h3>
          {#each plan.verifications as verification, i (i)}
            <div class={cardClass}>
              <p class={css({ fontSize: '13px', fontWeight: 'medium', marginBottom: '4px' })}>
                {verification.question}
                {#each verification.tools as tool, j (j)}
                  <span class={css({ marginLeft: '4px' })}><span class={badgeClass}>{tool}</span></span>
                {/each}
              </p>
              {@render field('수행', verification.detail)}
              {@render field('결론', verification.conclusion)}
            </div>
          {/each}
        </section>
      {/if}

      {#if plan.reviewResponses.length > 0}
        <section class={sectionClass}>
          <h3 class={sectionTitleClass}>검수 응답 ({plan.reviewResponses.length})</h3>
          {#each plan.reviewResponses as response, i (i)}
            <div class={cardClass}>
              <p class={css({ fontSize: '13px', fontWeight: 'medium', marginBottom: '4px' })}>
                <span class={badgeClass}>{response.disposition}</span>
                {response.target}
              </p>
              <p class={fieldBodyClass}>{response.reason}</p>
            </div>
          {/each}
        </section>
      {/if}
    {/if}
  </div>
</Modal>
