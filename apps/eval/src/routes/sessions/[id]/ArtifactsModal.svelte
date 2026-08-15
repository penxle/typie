<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal, RingSpinner } from '@typie/ui/components';
  import { untrack } from 'svelte';
  import IconX from '~icons/lucide/x';
  import { ARTIFACT_LABELS, ARTIFACT_ORDER, ARTIFACT_STAGES, countOf, refTargets } from '$lib/feedback/artifacts.ts';
  import { STAGES } from '$lib/feedback/stages.ts';
  import ArtifactSection from './artifacts/ArtifactSection.svelte';
  import AudienceSection from './artifacts/AudienceSection.svelte';
  import ConditionSection from './artifacts/ConditionSection.svelte';
  import ExperienceSection from './artifacts/ExperienceSection.svelte';
  import InterpretationSection from './artifacts/InterpretationSection.svelte';
  import JudgmentSection from './artifacts/JudgmentSection.svelte';
  import MovementsSection from './artifacts/MovementsSection.svelte';
  import NarrationSection from './artifacts/NarrationSection.svelte';
  import RubricSection from './artifacts/RubricSection.svelte';
  import StylisticSection from './artifacts/StylisticSection.svelte';
  import type { ArtifactName, Artifacts, ParsedArtifact } from '$lib/feedback/artifacts.ts';

  type Props = { open: boolean; sessionId: string; roundNumber: number };
  let { open = $bindable(), sessionId, roundNumber }: Props = $props();

  type Loaded = { status: 'idle' } | { status: 'loading' } | { status: 'error' } | { status: 'ready'; artifacts: Artifacts };
  let loaded = $state<Loaded>({ status: 'idle' });

  const load = async () => {
    loaded = { status: 'loading' };
    try {
      const res = await fetch(`/sessions/${sessionId}/artifacts`);
      if (!res.ok) throw new Error(`artifacts ${res.status}`);
      const payload = (await res.json()) as { artifacts: Artifacts };
      loaded = { status: 'ready', artifacts: payload.artifacts };
    } catch {
      loaded = { status: 'error' };
    }
  };

  let body = $state<HTMLElement>();
  let active = $state<ArtifactName>(ARTIFACT_ORDER[0]);

  // 첫 열림에 한 번 걷는다 — 닫았다 다시 열어도 재요청하지 않고, 오류일 때만 「다시 시도」가 다시 걷는다.
  $effect(() => {
    if (!open) return;
    // 열 때마다 본문은 맨 위에서 시작하므로 현재 섹션도 첫 섹션으로 되돌린다(닫혀 있는 동안의 값은 지난 열림의 것).
    active = ARTIFACT_ORDER[0];
    if (untrack(() => loaded.status) === 'idle') void load();
  });

  const artifacts = $derived(loaded.status === 'ready' ? loaded.artifacts : null);
  const targets = $derived(artifacts ? refTargets(artifacts) : new Set<string>());
  const movementTitles = $derived(
    artifacts?.movements.status === 'ok'
      ? new Map(artifacts.movements.value.movements.filter((m) => m.id).map((m) => [m.id, m.title]))
      : new Map<string, string>(),
  );

  const sectionId = (name: ArtifactName) => `af-section-${name}`;

  // 레일은 파이프라인이다 — 단계 라벨(과정 화면과 같은 말) 아래 그 단계의 산출물이 서고, 한 줄기 스템이 첫 산출물에서
  // 마지막 산출물까지 이어진다: 산출물 순서가 곧 검토 순서라는 사실을 구조로 보인다.
  type RailRow = { kind: 'stage'; label: string } | { kind: 'item'; name: ArtifactName };
  const rows: RailRow[] = STAGES.flatMap((stage): RailRow[] => {
    const names = ARTIFACT_ORDER.filter((name) => ARTIFACT_STAGES[name] === stage.key);
    return names.length === 0 ? [] : [{ kind: 'stage', label: stage.label }, ...names.map((name): RailRow => ({ kind: 'item', name }))];
  });
  const firstItem = rows.findIndex((row) => row.kind === 'item');
  const lastItem = rows.findLastIndex((row) => row.kind === 'item');
  const stemOf = (index: number): 'none' | 'start' | 'mid' | 'end' => {
    if (index < firstItem || index > lastItem) return 'none';
    if (index === firstItem) return 'start';
    if (index === lastItem) return 'end';
    return 'mid';
  };

  const stageOf = (name: ArtifactName): string => STAGES.find((stage) => stage.key === ARTIFACT_STAGES[name])?.label ?? '';

  // 스크롤스파이 — 본문 스크롤 위치 기준으로 헤더가 지나간 마지막 섹션이 현재 섹션이다. 끝까지 내려가면 마지막
  // 섹션이 짧아 헤더가 위에 닿지 못해도 그 섹션을 현재로 본다.
  const spy = () => {
    if (!body) return;
    if (body.scrollTop + body.clientHeight >= body.scrollHeight - 2) {
      active = ARTIFACT_ORDER.at(-1) ?? ARTIFACT_ORDER[0];
      return;
    }
    const line = body.scrollTop + 40;
    let current: ArtifactName = ARTIFACT_ORDER[0];
    for (const name of ARTIFACT_ORDER) {
      const el = body.querySelector<HTMLElement>(`#${sectionId(name)}`);
      if (el && el.offsetTop <= line) current = name;
    }
    active = current;
  };

  const go = (name: ArtifactName) => {
    const el = body?.querySelector<HTMLElement>(`#${sectionId(name)}`);
    if (!el || !body) return;
    body.scrollTo({ top: el.offsetTop - 24, behavior: 'smooth' });
    active = name;
  };

  const available = (name: ArtifactName): boolean => artifacts?.[name].status === 'ok';

  // 스템은 행마다 한 토막씩 — 첫 산출물은 점부터 아래로, 마지막 산출물은 점까지, 그 사이(단계 라벨 포함)는 관통.
  const stem = css.raw({
    position: 'relative',
    _before: { content: '""', position: 'absolute', left: '15px', top: '0', bottom: '0', width: '1px', backgroundColor: 'border.default' },
  });
  const stemVariants = {
    none: css.raw({ _before: { display: 'none' } }),
    start: css.raw({ _before: { top: '[50%]' } }),
    mid: css.raw({}),
    end: css.raw({ _before: { bottom: '[50%]' } }),
  };

  const railItem = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '10px',
    width: 'full',
    paddingY: '6px',
    paddingLeft: '12px',
    paddingRight: '8px',
    borderRadius: '6px',
    textAlign: 'left',
    fontSize: '13px',
    color: 'text.subtle',
    transition: 'common',
    transitionDuration: '[160ms]',
    transitionTimingFunction: '[cubic-bezier(0.23, 1, 0.32, 1)]',
    _hover: { color: 'text.default' },
    _active: { transform: '[scale(0.98)]' },
  });

  const dot = css.raw({
    position: 'relative',
    flex: 'none',
    size: '7px',
    borderRadius: 'full',
    backgroundColor: 'border.default',
    transition: 'colors',
    transitionDuration: '[160ms]',
  });
</script>

{#snippet unavailable(status: ParsedArtifact<unknown>['status'])}
  <p class={css({ fontSize: '13px', lineHeight: '[1.7]', color: 'text.faint' })}>
    {status === 'missing' ? '이 산출물이 없어요 — 이 기록이 생기기 전에 끝난 리뷰예요.' : '정리해서 보여드릴 수 없는 형식이에요.'}
  </p>
{/snippet}

<Modal style={css.raw({ width: 'full', maxWidth: '1040px', height: '[min(88vh, 900px)]', padding: '0', overflowY: 'hidden' })} bind:open>
  <div class={flex({ direction: 'column', height: 'full', minHeight: '0' })}>
    <header
      class={flex({
        align: 'center',
        gap: '8px',
        flex: 'none',
        height: '48px',
        paddingX: '16px',
        borderBottomWidth: '1px',
        borderColor: 'border.default',
      })}
    >
      <span class={css({ fontSize: '14px', fontWeight: 'semibold' })}>리뷰 산출물</span>
      <span class={css({ fontSize: '12px', color: 'text.faint' })}>·</span>
      <span class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>{roundNumber}회차</span>
      <button
        class={flex({
          align: 'center',
          justify: 'center',
          size: '28px',
          marginLeft: 'auto',
          borderRadius: '6px',
          color: 'text.subtle',
          transition: 'colors',
          transitionDuration: '[160ms]',
          _hover: { backgroundColor: 'surface.subtle', color: 'text.default' },
        })}
        aria-label="닫기"
        onclick={() => (open = false)}
        type="button"
      >
        <Icon icon={IconX} size={14} />
      </button>
    </header>

    <div class={flex({ flexGrow: '1', minHeight: '0' })}>
      <nav
        class={css({
          flex: 'none',
          width: '216px',
          paddingX: '12px',
          paddingY: '16px',
          borderRightWidth: '1px',
          borderColor: 'border.subtle',
          overflowY: 'auto',
        })}
        aria-label="산출물 목록"
      >
        {#each rows as row, index (index)}
          {#if row.kind === 'stage'}
            <div
              class={css(stem, stemVariants[stemOf(index)], {
                paddingLeft: '29px',
                paddingTop: index === 0 ? '0' : '14px',
                paddingBottom: '6px',
                fontSize: '11px',
                fontWeight: 'semibold',
                letterSpacing: '[0.02em]',
                color: 'text.faint',
              })}
            >
              {row.label}
            </div>
          {:else}
            {@const name = row.name}
            {@const count = artifacts ? countOf(name, artifacts) : null}
            {@const enabled = artifacts !== null}
            {@const isActive = active === name}
            {@const missing = enabled && !available(name)}
            <button
              class={css(
                stem,
                stemVariants[stemOf(index)],
                railItem,
                isActive && { color: 'text.default', fontWeight: 'semibold' },
                missing && { color: 'text.disabled', _hover: { color: 'text.faint' } },
                !enabled && { color: 'text.faint', pointerEvents: 'none' },
              )}
              aria-current={isActive ? 'true' : undefined}
              disabled={!enabled}
              onclick={() => go(name)}
              type="button"
            >
              <span
                class={css(
                  dot,
                  isActive && { backgroundColor: 'accent.brand.default', boxShadow: '[0 0 0 3px {colors.accent.brand.subtle}]' },
                  missing && { backgroundColor: 'surface.default', borderWidth: '1px', borderColor: 'border.default' },
                )}
              ></span>
              <span class={css({ minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}>
                {ARTIFACT_LABELS[name]}
              </span>
              {#if count !== null}
                <span
                  class={css({
                    flex: 'none',
                    marginLeft: 'auto',
                    fontFamily: 'mono',
                    fontSize: '11px',
                    letterSpacing: '0',
                    color: 'text.faint',
                  })}
                >
                  {count}
                </span>
              {/if}
            </button>
          {/if}
        {/each}
      </nav>

      <div
        bind:this={body}
        class={css({ position: 'relative', flexGrow: '1', minWidth: '0', overflowY: 'auto', paddingX: '40px', paddingY: '32px' })}
        onscroll={spy}
      >
        {#if loaded.status === 'loading' || loaded.status === 'idle'}
          <div class={flex({ align: 'center', justify: 'center', height: 'full' })}>
            <RingSpinner style={css.raw({ size: '28px', color: 'text.faint' })} />
          </div>
        {:else if loaded.status === 'error'}
          <div class={flex({ direction: 'column', align: 'center', justify: 'center', gap: '12px', height: 'full' })}>
            <p class={css({ fontSize: '13px', color: 'text.subtle' })}>산출물을 불러오지 못했어요</p>
            <Button onclick={load} size="sm" type="button" variant="secondary">다시 시도</Button>
          </div>
        {:else if artifacts}
          <div class={css({ maxWidth: '680px', paddingBottom: '64px' })}>
            <ArtifactSection id={sectionId('movements')} label={ARTIFACT_LABELS.movements} stage={stageOf('movements')}>
              {#if artifacts.movements.status === 'ok'}
                <MovementsSection value={artifacts.movements.value} />
              {:else}
                {@render unavailable(artifacts.movements.status)}
              {/if}
            </ArtifactSection>

            <ArtifactSection id={sectionId('narration')} label={ARTIFACT_LABELS.narration} stage={stageOf('narration')}>
              {#if artifacts.narration.status === 'ok'}
                <NarrationSection value={artifacts.narration.value} />
              {:else}
                {@render unavailable(artifacts.narration.status)}
              {/if}
            </ArtifactSection>

            <ArtifactSection id={sectionId('experience')} label={ARTIFACT_LABELS.experience} stage={stageOf('experience')}>
              {#if artifacts.experience.status === 'ok'}
                <ExperienceSection value={artifacts.experience.value} />
              {:else}
                {@render unavailable(artifacts.experience.status)}
              {/if}
            </ArtifactSection>

            <ArtifactSection id={sectionId('audience')} label={ARTIFACT_LABELS.audience} stage={stageOf('audience')}>
              {#if artifacts.audience.status === 'ok'}
                <AudienceSection value={artifacts.audience.value} />
              {:else}
                {@render unavailable(artifacts.audience.status)}
              {/if}
            </ArtifactSection>

            <ArtifactSection id={sectionId('condition')} label={ARTIFACT_LABELS.condition} stage={stageOf('condition')}>
              {#if artifacts.condition.status === 'ok'}
                <ConditionSection value={artifacts.condition.value} />
              {:else}
                {@render unavailable(artifacts.condition.status)}
              {/if}
            </ArtifactSection>

            <ArtifactSection id={sectionId('interpretation')} label={ARTIFACT_LABELS.interpretation} stage={stageOf('interpretation')}>
              {#if artifacts.interpretation.status === 'ok'}
                <InterpretationSection value={artifacts.interpretation.value} />
              {:else}
                {@render unavailable(artifacts.interpretation.status)}
              {/if}
            </ArtifactSection>

            <ArtifactSection id={sectionId('rubric')} label={ARTIFACT_LABELS.rubric} stage={stageOf('rubric')}>
              {#if artifacts.rubric.status === 'ok'}
                <RubricSection {targets} value={artifacts.rubric.value} />
              {:else}
                {@render unavailable(artifacts.rubric.status)}
              {/if}
            </ArtifactSection>

            <ArtifactSection id={sectionId('judgment')} label={ARTIFACT_LABELS.judgment} stage={stageOf('judgment')}>
              {#if artifacts.judgment.status === 'ok'}
                <JudgmentSection {targets} value={artifacts.judgment.value} />
              {:else}
                {@render unavailable(artifacts.judgment.status)}
              {/if}
            </ArtifactSection>

            <ArtifactSection id={sectionId('stylistic')} label={ARTIFACT_LABELS.stylistic} stage={stageOf('stylistic')}>
              {#if artifacts.stylistic.status === 'ok'}
                <StylisticSection {movementTitles} {targets} value={artifacts.stylistic.value} />
              {:else}
                {@render unavailable(artifacts.stylistic.status)}
              {/if}
            </ArtifactSection>
          </div>
        {/if}
      </div>
    </div>
  </div>
</Modal>
