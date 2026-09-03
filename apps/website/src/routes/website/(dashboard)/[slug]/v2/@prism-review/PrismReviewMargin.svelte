<script lang="ts">
  import { createMutation, createQuery, createSubscription } from '@mearie/svelte';
  import { TypieError } from '@typie/lib/errors';
  import { Toast } from '@typie/ui/notification';
  import { onDestroy, untrack } from 'svelte';
  import { Tween } from 'svelte/motion';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { cache, unwrapError } from '$lib/graphql';
  import { takeMarginJump } from '$lib/prism/margin-jump.svelte';
  import { readReviewRoundSelection, writeReviewRoundSelection } from '$lib/prism/review-round-selection';
  import { graphql } from '$mearie';
  import { fadeIn, fadeOut, PRISM_VISIBILITY_MOTION, prismVisibilityEasing, reducedMotion } from '../../../@prism/lib/motion.ts';
  import { TIER_OPTIONS } from '../../../@prism/review/tiers.ts';
  import { getZenMode } from '../../../zen-mode.svelte';
  import { getPane } from '../../@pane/context.svelte';
  import { setupMarginContext } from './context.svelte.ts';
  import {
    contentMotionOffset,
    marginInsets,
    marginMotionDuration,
    marginMotionTarget,
    nextMarginReserved,
    resolveRoundSwap,
  } from './margin-motion.ts';
  import { describeThread, resolveMode } from './margin-view.ts';
  import PrismReviewHighlightLayer from './PrismReviewHighlightLayer.svelte';
  import type { DataOf } from '@mearie/svelte';
  import type { StableSelection } from '@typie/editor-ffi/browser';
  import type { Snippet } from 'svelte';
  import type { MarginJump } from '$lib/prism/margin-jump.svelte';
  import type { DocumentPrismReviewMargin_Round_Query } from '$mearie';
  import type { DetailRound } from '../../../@prism/review/round-view.ts';
  import type { ZenModeReviewParticipant } from '../../../zen-mode.svelte';
  import type { MarginActivation, MarginItem, MarginPlacement, MarginSegment } from './context.svelte.ts';
  import type { RoundSwapState } from './margin-motion.ts';
  import type { MarginMode, RoundOption } from './margin-view.ts';

  // 인셋은 모드에 따라 정해지므로 자식에게 인자로 넘긴다 — DocumentEditor가 그대로 EditorComponent에 준다
  // 툴바까지 감싸려면 문서가 뜨기 전에도 마운트돼야 한다 — 그때 id는 null이고 컨트롤러는 빈 상태로 선다
  type Props = {
    documentId: string | null;
    entityId: string | null;
    myId: string;
    available: number;
    bodyWidth: number;
    children: Snippet<
      [
        {
          left: number;
          right: number;
          contentMotion?: { fromX: number; duration: number; easing: string };
        },
      ]
    >;
  };
  let { documentId, entityId, myId, available, bodyWidth, children }: Props = $props();

  type Seat = { id: string; selection: StableSelection; tone: 'issue' | 'strength' };
  type ReviewRound = DataOf<DocumentPrismReviewMargin_Round_Query>['prismReviewRound'];
  type Spec = {
    anchors: readonly { selection?: unknown }[];
    tone: 'issue' | 'strength';
    rangeId: (at: number) => string;
    item: Omit<MarginPlacement, 'rangeIds'>;
  };

  const ctx = getEditorContext();
  const pane = getPane();
  const zenMode = getZenMode();
  const editor = $derived(ctx.editor);
  const idle = $derived(documentId === null || entityId === null);

  let mode = $state<MarginMode>('popover');
  $effect(() => {
    mode = resolveMode(
      available,
      bodyWidth,
      untrack(() => mode),
    );
  });

  let ready = $state(false);

  const roundsQuery = createQuery(
    graphql(`
      query DocumentPrismReviewMargin_Rounds_Query($entityId: ID!) {
        entity(entityId: $entityId) {
          id
          slug

          node {
            __typename

            ... on Document {
              id
              title

              prismReviewRounds {
                id
                ordinal
                tier
                issueCount
                hasDetail
                sessionId
                createdAt

                lineage {
                  id
                  locked
                }
              }
            }
          }
        }
      }
    `),
    () => ({ entityId: entityId ?? '' }),
    () => ({ skip: entityId === null }),
  );

  const node = $derived(roundsQuery.data?.entity.node);

  const rounds = $derived<RoundOption[]>(
    (idle || node?.__typename !== 'Document' ? [] : node.prismReviewRounds).map((round) => ({
      id: round.id,
      ordinal: round.ordinal,
      tierLabel: TIER_OPTIONS.find((option) => option.tier === round.tier.toLowerCase())?.label ?? '리뷰',
      issueCount: round.issueCount,
      sessionId: round.sessionId ?? null,
      createdAt: round.createdAt,
      lineageId: round.lineage.id,
    })),
  );

  let selection = $state<string | null>();
  let selectedFor: string | null = null;

  // 저장된 값이 없거나 사라진 회차를 가리키면 최신 회차. 'none'은 작가가 명시적으로 끈 상태다.
  // 목록이 도착하기 전에 고르면 언제나 '없음'이 되므로 첫 응답을 기다린다.
  $effect(() => {
    const id = documentId;
    if (id === null || selectedFor === id || roundsQuery.data === undefined) return;
    const saved = readReviewRoundSelection(id);
    const known = untrack(() => rounds);
    selectedFor = id;
    selection = saved === 'none' ? null : (known.find((round) => round.id === saved)?.id ?? known[0]?.id ?? null);
  });

  const selectedRoundId = $derived(selection === undefined ? null : rounds.some((r) => r.id === selection) ? selection : null);

  let zenParticipant = $state.raw<ZenModeReviewParticipant | null>(null);
  $effect(() => {
    const id = documentId;
    if (id === null) {
      zenParticipant = null;
      return;
    }

    const participant: ZenModeReviewParticipant = {
      paneId: pane.id,
      documentId: id,
      ready: () => selectedFor === id && selection !== undefined,
      selectedRoundId: () => selectedRoundId,
      roundIds: () => rounds.map((round) => round.id),
      applySelection: (roundId) => {
        if (documentId === id) selection = roundId;
      },
    };
    zenParticipant = participant;
    const unregister = zenMode.registerReview(participant);
    return () => {
      unregister();
      if (zenParticipant === participant) zenParticipant = null;
    };
  });

  $effect(() => {
    void selection;
    void rounds;
    const participant = zenParticipant;
    if (participant) untrack(() => zenMode.syncReview(participant));
  });

  // 리뷰가 없는 문서에까지 여백을 잡아 두면 본문이 통째로 밀린다 — 회차가 걸린 뒤에만 자리를 낸다.
  // 짚은 곳이 하나도 없는 회차도 컬럼은 선다: 자리가 아예 없으면 "짚은 곳이 없어요"를 말할 자리도 없다.
  // 한 번 낸 자리는 회차가 걸려 있는 동안 유지한다: 회차를 갈아탈 때마다 접었다 펴면 본문이 좌우로 두 번 튄다.
  let reserved = $state(false);
  $effect(() => {
    reserved = nextMarginReserved(
      untrack(() => reserved),
      selectedRoundId,
      ready,
    );
  });

  const reduceMotion = reducedMotion();
  const presentation = new Tween(0, {
    duration: marginMotionDuration(reduceMotion),
    easing: prismVisibilityEasing,
  });
  let presentationPrepared = $state(false);
  // 이미 보이는 컬럼은 닫는 중 다시 열거나 회차를 바꿀 때 재입장을 기다리지 않는다.
  // 완전히 닫힌 컬럼만 첫 카드 배치가 끝났다는 신호를 받은 뒤 열림을 시작한다.
  const presentationAdmitted = $derived(presentationPrepared || presentation.current > 0);
  const presentationTarget = $derived(marginMotionTarget(mode, idle, reserved, presentationAdmitted));
  const openInsets = marginInsets(1);
  const contentShift = (openInsets.right - openInsets.left) / 2;
  let contentMotion = $state<{ fromX: number; duration: number; easing: string }>();
  // 목표가 바뀌는 순간의 진행률만 잡는다. Tween 매 프레임을 따라가면 CSS animation이 계속 다시 시작된다.
  $effect(() => {
    const target = presentationTarget;
    const progress = untrack(() => presentation.current);
    contentMotion =
      reduceMotion || progress === target
        ? undefined
        : {
            fromX: contentMotionOffset(target, progress, contentShift),
            duration: PRISM_VISIBILITY_MOTION.duration,
            easing: PRISM_VISIBILITY_MOTION.easing,
          };
    presentation.target = target;
  });
  $effect(() => {
    if (presentation.current === presentationTarget) contentMotion = undefined;
  });

  $effect(() => {
    if (mode !== 'column') presentationPrepared = false;
  });

  // 패널 열림/닫힘과 회차 교체는 서로 다른 수명이다. 회차 교체 중에는 기존 결과를 fade-out 끝까지
  // 붙잡고, 새 데이터와 카드 배치가 모두 준비된 뒤에만 전체 표면을 다시 드러낸다.
  let presentedRound = $state.raw<ReviewRound | null>(null);
  let presentedFor: string | null = null;
  let roundSwap = $state.raw<RoundSwapState>({ phase: 'idle' });
  const roundVisibility = new Tween(1);

  const select = (roundId: string | null) => {
    selection = roundId;
    if (documentId !== null) writeReviewRoundSelection(documentId, roundId);
  };

  // 총평이 없는 회차(hasDetail=false)는 열 문이 없어야 한다 — 세션 카드의 게이트와 같은 기준이다
  const detailRound = $derived.by((): DetailRound | null => {
    const entity = roundsQuery.data?.entity;
    if (idle || entity === undefined || entity.node.__typename !== 'Document') return null;
    const selected = entity.node.prismReviewRounds.find((round) => round.id === presentedRound?.id);
    if (selected === undefined || !selected.hasDetail) return null;
    return {
      id: selected.id,
      tier: selected.tier,
      ordinal: selected.ordinal,
      issueCount: selected.issueCount,
      document: { id: entity.node.id, title: entity.node.title, entity: { slug: entity.slug } },
    };
  });

  const detailQuery = createQuery(
    graphql(`
      query DocumentPrismReviewMargin_Round_Query($roundId: ID!) {
        prismReviewRound(roundId: $roundId) {
          id
          ordinal
          issueCount
          threads {
            id
            issueIndex(roundId: $roundId)
            trait
            body
            quote(roundId: $roundId)
            state
            reaction
            isNew(roundId: $roundId)
            anchors(roundId: $roundId) {
              selection
            }
            comments {
              id
              author
              body
              createdAt

              user {
                id
                name

                avatar {
                  id
                  ...Img_image
                }
              }
            }
          }
          settledThreads {
            id
            issueIndex(roundId: $roundId)
            trait
            body
            quote(roundId: $roundId)
            state
            reaction
            isNew(roundId: $roundId)
            anchors(roundId: $roundId) {
              selection
            }
            comments {
              id
              author
              body
              createdAt

              user {
                id
                name

                avatar {
                  id
                  ...Img_image
                }
              }
            }
          }
          lineage {
            id
            locked
          }
          detail {
            strengths {
              quote
              body
              anchors {
                selection
              }
            }
            patterns {
              theme
              body
              issues {
                index
                trait
              }
            }
            priorities {
              body
              issues {
                index
                trait
              }
            }
          }
        }
      }
    `),
    () => ({ roundId: selectedRoundId ?? '' }),
    () => ({ skip: selectedRoundId === null }),
  );

  createSubscription(
    graphql(`
      subscription DocumentPrismReviewMargin_Stream($documentId: ID!) {
        prismReviewStream(documentId: $documentId) {
          id
          ordinal
          issueCount
        }
      }
    `),
    () => ({ documentId: documentId ?? '' }),
    () => ({
      skip: documentId === null,
      onData: (data) => {
        const current = documentId;
        if (current === null) return;
        const arrived = data.prismReviewStream.id;
        // 스레드 뮤테이션마다도 같은 채널로 발행된다 — 내 편집까지 전량 재조회하지 않도록 무효화로 받는다
        cache.invalidate(
          { __typename: 'Document', id: current, $field: 'prismReviewRounds' },
          { __typename: 'Document', id: current, $field: 'prismReviewLineages' },
          { __typename: 'PrismReviewRound', id: arrived, $field: 'threads' },
          { __typename: 'PrismReviewRound', id: arrived, $field: 'settledThreads' },
          { __typename: 'PrismReviewRound', id: arrived, $field: 'detail' },
        );
        // 목록을 모르는 동안에는 무엇이 새 회차인지 가릴 수 없다 — 가리기 전에 고르면 옛 회차가 저장된다
        if (roundsQuery.data === undefined) return;
        // 새 결과가 도착하면 꺼 둔 문서라도 그 회차를 켠다 — 작가가 방금 부탁한 리뷰다
        if (rounds.every((round) => round.id !== arrived)) select(arrived);
      },
    }),
  );

  // 쿼리는 목표 회차를 미리 불러오지만, 실제 표시 데이터의 교체는 fade-out 경계가 소유한다.
  const loadedRound = $derived.by(() => {
    const detail = detailQuery.data?.prismReviewRound;
    return detail !== undefined && detail.id === selectedRoundId ? detail : null;
  });

  $effect(() => {
    const id = documentId;
    if (presentedFor === id) return;
    untrack(() => {
      presentedFor = id;
      presentedRound = null;
      roundSwap = { phase: 'idle' };
      void roundVisibility.set(1, { duration: 0 });
    });
  });

  // 첫 표시와 완전 닫힘은 컬럼 presentation이 맡는다. 같은 회차의 갱신은 즉시 받되,
  // 서로 다른 회차의 교체만 아래 roundSwap 상태가 fade-out 경계에서 수행한다.
  $effect(() => {
    const selected = selectedRoundId;
    const loaded = loadedRound;
    const current = presentedRound;
    const progress = presentation.current;
    untrack(() => {
      if (selected === null) {
        if (progress === 0) presentedRound = null;
      } else if (current !== loaded && loaded?.id === selected && (current === null || current.id === selected)) {
        presentedRound = loaded;
      }
    });
  });

  const round = $derived(presentedRound);

  // 그 계보의 다음 리뷰가 도는 동안에는 답글·닫기가 사영의 입력을 바꾼다 — 회차 경계 밖에서 잠근다
  const locked = $derived(round?.lineage.locked ?? false);

  // 응답 객체의 신원이 아니라 실제 카드 자리 재계산에 쓰는 값만 본다. 댓글·상태 갱신이나
  // 같은 네트워크 응답이 다시 와도 재앵커링과 WASM 좌표 변환을 반복하지 않는다.
  const placementKey = $derived.by(() =>
    round === null
      ? null
      : JSON.stringify([
          round.id,
          round.threads.map((thread) => [thread.id, thread.issueIndex, thread.anchors]),
          (round.detail?.strengths ?? []).map((strength) => strength.anchors),
          (round.detail?.patterns ?? []).map((pattern) => [pattern.theme, pattern.issues.map((issue) => issue.index)]),
          (round.detail?.priorities ?? []).map((priority) => [priority.body, priority.issues.map((issue) => issue.index)]),
        ]),
  );

  let activation = $state<MarginActivation>({ id: null, rangeId: null, sequence: 0 });
  const activeId = $derived(activation.id);

  let raw = $state.raw<MarginPlacement[]>([]);
  // 지금 선 항목이 어느 회차의 것인지 — 회차를 갈아 끼운 직후엔 고른 회차와 어긋난다
  let builtRoundId = $state<string | null>(null);
  // clear/add를 요청한 판보다 새 editor 판이 적용되어야 새 앵커의 자리를 준비했다고 볼 수 있다.
  let rangeInstallation = $state.raw<{ roundId: string; previousRevision: number } | null>(null);

  // 앵커는 서버가 리뷰 시점에 캡처한 selection이라 회차당 정적이다 — 회차 전환·데이터 도착 때만 전량 재설치하고,
  // 편집 뒤의 자리는 코어가 매 판 해소한다(자리를 잃은 range는 tracked_ranges()에서 빠지고, 되돌리기로 돌아오면 다시 선다).
  const buildItems = () => {
    const current = editor;
    if (round === null || !current || current.terminal) {
      current?.clearPrismReviewRanges();
      raw = [];
      ready = false;
      builtRoundId = null;
      rangeInstallation = null;
      presentationPrepared = false;
      return;
    }

    if (builtRoundId !== round.id) presentationPrepared = false;

    // 코드젠은 nullable을 `?: T | null`로 내므로 undefined를 null로 접어 문면 타입에 맞춘다
    const patterns = (round.detail?.patterns ?? []).map((pattern) => ({ ...pattern, theme: pattern.theme ?? null }));
    const priorities = round.detail?.priorities ?? [];
    const strengths = round.detail?.strengths ?? [];

    const specs: Spec[] = [
      ...round.threads.map((thread) => ({
        anchors: thread.anchors,
        tone: 'issue' as const,
        rangeId: (at: number) => `${round.id}:${thread.id}:${at}`,
        item: {
          id: thread.id,
          kind: 'issue' as const,
          number: thread.issueIndex + 1,
          callouts: describeThread(thread.issueIndex, patterns, priorities),
          strengthIndex: null,
        },
      })),
      ...strengths.map((strength, index) => ({
        anchors: strength.anchors,
        tone: 'strength' as const,
        rangeId: (at: number) => `${round.id}:strength:${index}:${at}`,
        item: {
          id: `strength:${index}`,
          kind: 'strength' as const,
          number: index + 1,
          callouts: { pattern: null, priority: null },
          strengthIndex: index,
        },
      })),
    ];

    const installs: Seat[] = [];
    const nextRaw: MarginPlacement[] = [];
    for (const spec of specs) {
      const rangeIds: string[] = [];
      for (const [at, anchor] of spec.anchors.entries()) {
        if (anchor.selection === null || anchor.selection === undefined) continue;
        const id = spec.rangeId(at);
        rangeIds.push(id);
        installs.push({ id, selection: anchor.selection as StableSelection, tone: spec.tone });
      }
      nextRaw.push({ ...spec.item, rangeIds });
    }

    const previousRevision = current.appliedRevision;
    current.setPrismReviewRanges(installs);
    raw = nextRaw;
    ready = true;
    builtRoundId = round.id;
    rangeInstallation = { roundId: round.id, previousRevision };
  };

  const markPresentationPrepared = (roundId: string) => {
    if (mode !== 'column' || !ready || selectedRoundId !== roundId || builtRoundId !== roundId) return;
    presentationPrepared = true;
  };

  // `add`는 결과 이벤트가 없다 — 실제로 앉았는지는 적용된 스냅숏에 나타나는지로만 안다.
  // 편집으로 range가 떨어져 나가는 것도 같은 자리에서 잡힌다.
  const items = $derived.by(() => {
    const threadById = new Map((round?.threads ?? []).map((thread) => [thread.id, thread]));
    const strengths = round?.detail?.strengths ?? [];

    const join = (place: MarginPlacement, anchored: boolean): MarginItem => {
      const index = place.strengthIndex;
      const found = index === null ? undefined : strengths[index];
      return {
        ...place,
        anchored,
        thread: threadById.get(place.id) ?? null,
        strength: index === null || found === undefined ? null : { index, quote: found.quote, body: found.body ?? null },
      };
    };

    const current = editor;
    if (!current) return raw.map((place) => join(place, false));
    // 스냅숏 사본의 trackedRanges는 코어가 그 필드를 낼 때만 갈린다 — 문단을 지워 range가 빠져도
    // 사본에는 남아 자리 잃음이 다음 새로고침까지 드러나지 않는다. 판이 갈릴 때마다 지금 것을 받는다.
    void current.appliedSnapshot.revision;
    const alive = new Set(current.freshTrackedRanges().map((range) => range.id));
    return raw.map((place) =>
      join(
        place,
        place.rangeIds.some((id) => alive.has(id)),
      ),
    );
  });

  $effect(() => {
    void editor;
    void placementKey;
    untrack(() => buildItems());
  });

  const rangesApplied = $derived(
    editor !== undefined &&
      rangeInstallation !== null &&
      rangeInstallation.roundId === builtRoundId &&
      editor.appliedRevision > rangeInstallation.previousRevision,
  );
  const selectedRoundPrepared = $derived(
    selectedRoundId !== null && builtRoundId === selectedRoundId && ready && rangesApplied && (mode !== 'column' || presentationPrepared),
  );
  const roundLoading = $derived(roundSwap.phase === 'waiting' || (roundSwap.phase === 'preparing' && roundSwap.spinnerVisible));
  const roundInteractive = $derived(
    roundSwap.phase === 'idle' && selectedRoundPrepared && roundVisibility.target === 1 && roundVisibility.current === 1,
  );

  $effect(() => {
    const state = roundSwap;
    const selected = selectedRoundId;
    const presented = presentedRound;
    const loaded = loadedRound;
    const resolution = resolveRoundSwap(state, {
      selectedRoundId: selected,
      presentedRoundId: presented?.id ?? null,
      loadedRoundId: loaded?.id ?? null,
      visibilityProgress: roundVisibility.current,
      prepared: selectedRoundPrepared,
      failed: detailQuery.error !== undefined,
    });

    untrack(() => {
      if (resolution.state !== state) roundSwap = resolution.state;
      if (resolution.replacePresented && loaded?.id === selected) presentedRound = loaded;

      if (presented !== null && resolution.restoreSelection) {
        select(presented.id);
        Toast.error('리뷰를 불러오지 못했어요. 잠시 후 다시 시도해 주세요.');
      }

      if (roundVisibility.target !== resolution.visibilityTarget) {
        const options = resolution.visibilityTarget === 0 ? fadeOut : fadeIn;
        void roundVisibility.set(resolution.visibilityTarget, { ...options, duration: reduceMotion ? 0 : options.duration });
      }
    });
  });

  // 닫는다고 목록에서 빠지지 않는다 — 이번 회차의 피드백은 닫혀도 회색으로 제자리에 남는다.
  // 다른 갈래로 옮겨 가는 것은 재리뷰 사영(회차 경계)에서만 일어난다.
  const issues = $derived(items.filter((item) => item.kind === 'issue'));
  const lostCards = $derived(issues.filter((item) => !item.anchored));
  const openCards = $derived(issues.filter((item) => item.anchored));

  // 정리된 스레드는 앵커 없이 컬럼 하단에 흐름으로 선다 — 이 회차의 사영이 해소·철회한 것들이다
  const settledCards = $derived<MarginItem[]>(
    (round?.settledThreads ?? []).map((thread) => ({
      id: thread.id,
      kind: 'issue',
      number: thread.issueIndex + 1,
      rangeIds: [],
      callouts: { pattern: null, priority: null },
      strengthIndex: null,
      anchored: false,
      thread,
      strength: null,
    })),
  );

  let segment = $state<MarginSegment>('open');
  const setSegment = (next: MarginSegment) => {
    segment = next;
  };

  const segmentCards = $derived(segment === 'lost' ? lostCards : segment === 'settled' ? settledCards : openCards);

  // 레일·룰러는 세그먼트를 따르지 않는다(오너 결정) — 대신 다른 갈래의 카드를 켜면 목록이 그 카드를 따라간다
  const segmentOf = (id: string): MarginSegment | null => {
    if (lostCards.some((item) => item.id === id)) return 'lost';
    if (settledCards.some((item) => item.id === id)) return 'settled';
    return openCards.some((item) => item.id === id) ? 'open' : null;
  };

  $effect(() => {
    const id = activeId;
    if (id === null) return;
    untrack(() => {
      const owner = segmentOf(id);
      if (owner !== null && owner !== segment) setSegment(owner);
    });
  });

  const activate = (id: string | null) => {
    const sequence = activation.sequence + 1;
    if (id === null) {
      activation = { id: null, rangeId: null, sequence };
      return;
    }
    const current = editor;
    const target = items.find((item) => item.id === id);
    // 죽은 id로는 아무 데도 가지 못한다 — 코어가 아직 들고 있는 자리로만 데려간다
    const live = new Set(current?.appliedSnapshot.trackedRanges.map((range) => range.id));
    const rangeId = target?.anchored ? (target.rangeIds.find((candidate) => live.has(candidate)) ?? null) : null;
    activation = { id, rangeId, sequence };
    if (!current || target === undefined || rangeId === null) return;

    // 컬럼 지적은 최종 높이와 배치 시점을 소유하는 PrismCardColumn이 reveal한다.
    if (mode === 'column' && target.kind === 'issue') return;

    void current.revealTrackedItem(rangeId);
  };

  $effect(() => {
    void selectedRoundId;
    untrack(() => {
      activation = { id: null, rangeId: null, sequence: activation.sequence + 1 };
    });
  });

  // 대화·총평에서 온 요청은 회차부터 갈아 끼워야 한다 — 그 회차의 항목은 응답이 와서 다시 앉은 뒤에야 선다
  let pendingJump = $state<MarginJump | null>(null);

  $effect(() => {
    const requested = takeMarginJump(documentId);
    if (requested === null) return;
    untrack(() => {
      if (requested.roundId !== selection) select(requested.roundId);
      pendingJump = requested.itemId === null ? null : requested;
    });
  });

  $effect(() => {
    const target = pendingJump;
    if (target === null) return;

    // 그 사이 작가가 회차를 옮겼으면 갈 자리가 없다
    if (selection !== target.roundId) {
      untrack(() => {
        pendingJump = null;
      });
      return;
    }

    // 강점 id는 회차마다 되풀이된다 — 아직 옛 회차의 항목이 서 있는 동안 엉뚱한 곳으로 가지 않도록 회차까지 본다
    if (!selectedRoundPrepared || builtRoundId !== target.roundId || items.every((item) => item.id !== target.itemId)) return;

    untrack(() => {
      activate(target.itemId);
      pendingJump = null;
    });
  });

  const [replyMutation] = createMutation(
    graphql(`
      mutation DocumentPrismReviewMargin_Reply($input: ReplyPrismReviewThreadInput!) {
        replyPrismReviewThread(input: $input) {
          id
          comments {
            id
            author
            body
            createdAt

            user {
              id
              name

              avatar {
                id
                ...Img_image
              }
            }
          }
        }
      }
    `),
  );

  const [updateCommentMutation] = createMutation(
    graphql(`
      mutation DocumentPrismReviewMargin_UpdateComment($input: UpdatePrismReviewThreadCommentInput!) {
        updatePrismReviewThreadComment(input: $input) {
          id
          comments {
            id
            author
            body
            createdAt

            user {
              id
              name

              avatar {
                id
                ...Img_image
              }
            }
          }
        }
      }
    `),
  );

  const [deleteCommentMutation] = createMutation(
    graphql(`
      mutation DocumentPrismReviewMargin_DeleteComment($input: DeletePrismReviewThreadCommentInput!) {
        deletePrismReviewThreadComment(input: $input) {
          id
          comments {
            id
            author
            body
            createdAt

            user {
              id
              name

              avatar {
                id
                ...Img_image
              }
            }
          }
        }
      }
    `),
  );

  const [closeMutation] = createMutation(
    graphql(`
      mutation DocumentPrismReviewMargin_Close($input: ClosePrismReviewThreadInput!) {
        closePrismReviewThread(input: $input) {
          id
          state
          stateChangedAt
        }
      }
    `),
  );

  const [reopenMutation] = createMutation(
    graphql(`
      mutation DocumentPrismReviewMargin_Reopen($input: ReopenPrismReviewThreadInput!) {
        reopenPrismReviewThread(input: $input) {
          id
          state
          stateChangedAt
        }
      }
    `),
  );

  const [reactMutation] = createMutation(
    graphql(`
      mutation DocumentPrismReviewMargin_React($input: ReactPrismReviewThreadInput!) {
        reactPrismReviewThread(input: $input) {
          id
          reaction
        }
      }
    `),
  );

  const guard = async (run: () => Promise<unknown>, message: string) => {
    try {
      await run();
    } catch (err) {
      // 잠금은 서버가 지킨다 — 여백이 잠금을 늦게 알아도 막힌 이유는 말해 준다
      const error = unwrapError(err);
      const code = error instanceof TypieError ? error.code : null;
      Toast.error(code === 'prism_review_running' ? '리뷰가 진행 중이에요' : message);
      throw err;
    }
  };

  setupMarginContext({
    get rounds() {
      return rounds;
    },
    get selectedRoundId() {
      return selectedRoundId;
    },
    get detailRound() {
      return detailRound;
    },
    get items() {
      return items;
    },
    get segment() {
      return segment;
    },
    get segmentCounts() {
      return { open: openCards.length, settled: settledCards.length, lost: lostCards.length };
    },
    get segmentCards() {
      return segmentCards;
    },
    get activation() {
      return activation;
    },
    get activeId() {
      return activeId;
    },
    get mode() {
      return mode;
    },
    get ready() {
      return ready;
    },
    get presentationRoundId() {
      return builtRoundId;
    },
    get presentationProgress() {
      return presentation.current;
    },
    get roundVisibilityProgress() {
      return roundVisibility.current;
    },
    get roundLoading() {
      return roundLoading;
    },
    get roundInteractive() {
      return roundInteractive;
    },
    get presentationInteractive() {
      return presentationTarget === 1 && presentation.current === 1 && roundInteractive;
    },
    get myId() {
      return myId;
    },
    get locked() {
      return locked;
    },
    select,
    activate,
    markPresentationPrepared,
    setSegment,
    reply: (threadId, body) => guard(() => replyMutation({ input: { threadId, body } }), '답글을 남기지 못했어요'),
    editComment: (commentId, body) => guard(() => updateCommentMutation({ input: { commentId, body } }), '처리하지 못했어요'),
    deleteComment: (commentId) => guard(() => deleteCommentMutation({ input: { commentId } }), '처리하지 못했어요'),
    close: (threadId) => guard(() => closeMutation({ input: { threadId } }), '처리하지 못했어요'),
    reopen: (threadId) => guard(() => reopenMutation({ input: { threadId } }), '처리하지 못했어요'),
    react: (threadId, value) => guard(() => reactMutation({ input: { threadId, value } }), '처리하지 못했어요'),
  });

  const insets = $derived({ ...marginInsets(presentationTarget), contentMotion });

  onDestroy(() => {
    editor?.clearPrismReviewRanges();
  });
</script>

{@render children(insets)}
<PrismReviewHighlightLayer />
