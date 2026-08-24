<script lang="ts">
  import { createMutation, createQuery, createSubscription } from '@mearie/svelte';
  import { reanchorAll } from '@typie/prism';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { onDestroy, tick, untrack } from 'svelte';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { cache } from '$lib/graphql';
  import { takeMarginJump } from '$lib/prism/margin-jump.svelte';
  import { graphql } from '$mearie';
  import { TIER_OPTIONS } from '../../../@prism/review/tiers.ts';
  import { setupMarginContext } from './context.svelte.ts';
  import { COLUMN_GAP, COLUMN_WIDTH, describeThread, GUTTER, resolveMode } from './margin-view.ts';
  import type { Selection } from '@typie/editor-ffi/browser';
  import type { Anchor } from '@typie/prism';
  import type { Snippet } from 'svelte';
  import type { MarginJump } from '$lib/prism/margin-jump.svelte';
  import type { DetailRound } from '../../../@prism/review/round-view.ts';
  import type { MarginActivationSource, MarginItem, MarginPlacement, MarginSegment } from './context.svelte.ts';
  import type { MarginMode, RoundOption } from './margin-view.ts';

  // 인셋은 모드에 따라 정해지므로 자식에게 인자로 넘긴다 — DocumentEditor가 그대로 EditorComponent에 준다
  // 툴바까지 감싸려면 문서가 뜨기 전에도 마운트돼야 한다 — 그때 id는 null이고 컨트롤러는 빈 상태로 선다
  type Props = {
    documentId: string | null;
    entityId: string | null;
    myId: string;
    available: number;
    bodyWidth: number;
    children: Snippet<[{ left: number; right: number }]>;
  };
  let { documentId: documentIdProp, entityId: entityIdProp, myId, available, bodyWidth, children }: Props = $props();

  const app = getAppContext();

  // 접근이 없으면 id가 없는 상태로 접는다 — 기존 "문서 없음" 경로가 그대로 답이다.
  const documentId = $derived(app.state.prismAccess ? documentIdProp : null);
  const entityId = $derived(app.state.prismAccess ? entityIdProp : null);

  type Seat = { id: string; selection: Selection; tone: 'issue' | 'strength' };
  type Placed = { rangeIds: string[]; seats: Seat[] };
  type Spec = {
    anchors: readonly Anchor[];
    tone: 'issue' | 'strength';
    rangeId: (at: number) => string;
    item: Omit<MarginPlacement, 'rangeIds'>;
  };

  const ctx = getEditorContext();
  const editor = $derived(ctx.editor);
  const idle = $derived(documentId === null || entityId === null);

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
    })),
  );

  const storageKey = $derived(documentId === null ? null : `typie:prism-review-round:${documentId}`);
  let selection = $state<string | null>();
  let selectedFor: string | null = null;

  // 저장된 값이 없거나 사라진 회차를 가리키면 최신 회차. 'none'은 작가가 명시적으로 끈 상태다.
  // 목록이 도착하기 전에 고르면 언제나 '없음'이 되므로 첫 응답을 기다린다.
  $effect(() => {
    const key = storageKey;
    if (key === null || selectedFor === key || roundsQuery.data === undefined) return;
    const saved = localStorage.getItem(key);
    const known = untrack(() => rounds);
    selectedFor = key;
    selection = saved === 'none' ? null : (known.find((round) => round.id === saved)?.id ?? known[0]?.id ?? null);
  });

  const selectedRoundId = $derived(selection === undefined ? null : rounds.some((r) => r.id === selection) ? selection : null);

  const select = (roundId: string | null) => {
    selection = roundId;
    const key = storageKey;
    if (key !== null) localStorage.setItem(key, roundId ?? 'none');
  };

  // 총평이 없는 회차(hasDetail=false)는 열 문이 없어야 한다 — 세션 카드의 게이트와 같은 기준이다
  const detailRound = $derived.by((): DetailRound | null => {
    const entity = roundsQuery.data?.entity;
    if (idle || entity === undefined || entity.node.__typename !== 'Document') return null;
    const selected = entity.node.prismReviewRounds.find((round) => round.id === selectedRoundId);
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
            issueIndex
            trait
            body
            quote
            state
            reaction
            anchors {
              start
              end
              head
              tail
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
          detail {
            strengths {
              quote
              body
              anchor {
                start
                end
                head
                tail
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
          { __typename: 'PrismReviewRound', id: arrived, $field: 'threads' },
          { __typename: 'PrismReviewRound', id: arrived, $field: 'detail' },
        );
        // 목록을 모르는 동안에는 무엇이 새 회차인지 가릴 수 없다 — 가리기 전에 고르면 옛 회차가 저장된다
        if (roundsQuery.data === undefined) return;
        // 새 결과가 도착하면 꺼 둔 문서라도 그 회차를 켠다 — 작가가 방금 부탁한 리뷰다
        if (rounds.every((round) => round.id !== arrived)) select(arrived);
      },
    }),
  );

  // 고른 회차의 것이 아닌 응답은 쓰지 않는다 — 회차를 바꾼 직후 한 틱 동안 옛 회차가 그려진다
  const round = $derived.by(() => {
    const detail = detailQuery.data?.prismReviewRound;
    return detail !== undefined && detail.id === selectedRoundId ? detail : null;
  });

  let mode = $state<MarginMode>('popover');
  $effect(() => {
    mode = resolveMode(
      available,
      bodyWidth,
      untrack(() => mode),
    );
  });

  let activeId = $state<string | null>(null);

  let raw = $state.raw<MarginPlacement[]>([]);
  let ready = $state(false);
  // 지금 선 항목이 어느 회차의 것인지 — 회차를 갈아 끼운 직후엔 고른 회차와 어긋난다
  let builtRoundId = $state<string | null>(null);
  let placed = new Map<string, Placed>();

  // 재앵커링은 본문 길이에 비례하는 비용이라 타이핑 한 번에 전 항목을 다시 계산할 수 없다.
  // stale이 오면 그 이벤트가 죽인 항목만 다시 앉히고, 살아남은 자리는 코어가 들고 있는 현재 좌표로 그대로 옮긴다.
  const buildItems = (stale?: ReadonlySet<string>, retryLost = false) => {
    const current = editor;
    if (round === null || !current || current.terminal) {
      current?.clearPrismReviewRanges();
      placed = new Map();
      raw = [];
      ready = false;
      builtRoundId = null;
      return;
    }

    // 코드젠은 nullable을 `?: T | null`로 내므로 undefined를 null로 접어 문면 타입에 맞춘다
    const patterns = (round.detail?.patterns ?? []).map((pattern) => ({ ...pattern, theme: pattern.theme ?? null }));
    const priorities = round.detail?.priorities ?? [];
    const strengths = round.detail?.strengths ?? [];

    const specs: Spec[] = [
      ...round.threads.map((thread) => ({
        anchors: thread.anchors,
        tone: 'issue' as const,
        rangeId: (at: number) => `${thread.id}:${at}`,
        item: {
          id: thread.id,
          kind: 'issue' as const,
          number: thread.issueIndex + 1,
          callouts: describeThread(thread.issueIndex, patterns, priorities),
          strengthIndex: null,
        },
      })),
      ...strengths.map((strength, index) => ({
        anchors: [strength.anchor],
        tone: 'strength' as const,
        rangeId: () => `strength:${index}`,
        item: {
          id: `strength:${index}`,
          kind: 'strength' as const,
          number: index + 1,
          callouts: { pattern: null, priority: null },
          strengthIndex: index,
        },
      })),
    ];

    const redoing = specs.filter((spec) => {
      if (stale === undefined) return true;
      const prior = placed.get(spec.item.id);
      if (prior === undefined) return true;
      // 자리를 잃은 항목은 stale 집합에 다시 나타날 수 없다 — 되돌리기로 원문이 돌아와도 스스로는 못 깨어난다.
      // 그래서 다시 시도해야 하지만, 재앵커링은 원고 길이에 비례하는 비용이라 타건마다 돌릴 수 없다.
      // 이 복구는 타건이 멎은 뒤에만 선다(retryLost).
      if (prior.rangeIds.length === 0) return retryLost;
      return prior.rangeIds.some((id) => stale.has(id));
    });

    const resolved =
      redoing.length > 0
        ? reanchorAll(
            current.proseTextAnnotated(),
            redoing.flatMap((spec) => spec.anchors),
          )
        : [];

    // 자리 장부는 반응성 밖이다 — 화면은 raw·items가 그리고 이 맵은 다음 재앵커링의 입력일 뿐이다
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const next = stale === undefined ? new Map<string, Placed>() : new Map(placed);
    let cursor = 0;
    for (const spec of redoing) {
      const rangeIds: string[] = [];
      const seats: Seat[] = [];
      for (const at of spec.anchors.keys()) {
        const range = resolved[cursor + at];
        if (range === null) continue;
        const sel = current.proseToSelectionAnnotated(range.start, range.end);
        if (!sel) continue;
        const id = spec.rangeId(at);
        rangeIds.push(id);
        seats.push({ id, selection: sel, tone: spec.tone });
      }
      cursor += spec.anchors.length;
      next.set(spec.item.id, { rangeIds, seats });
    }

    const redone = new Set(redoing.map((spec) => spec.item.id));
    const alive = new Map(current.appliedSnapshot.trackedRanges.map((range) => [range.id, range]));

    const installs: Seat[] = [];
    const nextRaw: MarginPlacement[] = [];
    for (const spec of specs) {
      const place = next.get(spec.item.id);
      if (place === undefined) {
        nextRaw.push({ ...spec.item, rangeIds: [] });
        continue;
      }

      // 처음 넣은 좌표는 그 사이 편집만큼 밀려 있다 — 코어가 아직 들고 있는 자리는 그 좌표를 그대로 쓴다
      const seats = redone.has(spec.item.id)
        ? place.seats
        : place.seats.map((seat) => {
            const range = alive.get(seat.id);
            return range === undefined ? seat : { ...seat, selection: { anchor: range.anchor, head: range.head } };
          });

      next.set(spec.item.id, { rangeIds: place.rangeIds, seats });
      installs.push(...seats);
      nextRaw.push({ ...spec.item, rangeIds: place.rangeIds });
    }

    current.installPrismReviewDecorations();
    current.setPrismReviewRanges(installs);
    placed = next;
    raw = nextRaw;
    ready = true;
    builtRoundId = round.id;
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
    void round;
    untrack(() => buildItems());
  });

  // 지적받은 대목이 편집되면 코어가 range를 떨군다 — 그 지적만 한 번 다시 앉혀 본다
  $effect(() => {
    const current = editor;
    if (!current) return;
    return current.on('tracked_ranges_stale', (_, { ids }) => {
      const dead = new Set(ids);
      if (raw.some((item) => item.rangeIds.some((rangeId) => dead.has(rangeId)))) untrack(() => buildItems(dead));
      untrack(scheduleLostRetry);
    });
  });

  // 되돌리기로 원문이 돌아오면 자리를 잃은 지적이 다시 앉을 수 있다. 다만 그 판정은 원고 전문을 다시 훑는
  // 비용이라 타건 경로에 둘 수 없다 — 손이 멎은 뒤에 한 번만 돌린다.
  const LOST_RETRY_IDLE = 2000;
  let lostRetry: ReturnType<typeof setTimeout> | undefined;
  const scheduleLostRetry = () => {
    clearTimeout(lostRetry);
    if (raw.every((item) => item.rangeIds.length > 0)) return;
    lostRetry = setTimeout(() => untrack(() => buildItems(new Set(), true)), LOST_RETRY_IDLE);
  };

  $effect(() => {
    const current = editor;
    if (!current) return;
    const active = items.find((item) => item.id === activeId);
    current.setActivePrismReviewRanges(active?.rangeIds ?? []);
  });

  // 닫는다고 목록에서 빠지지 않는다 — 이번 회차의 피드백은 닫혀도 회색으로 제자리에 남는다.
  // 다른 갈래로 옮겨 가는 것은 재리뷰 사영(회차 경계)에서만 일어나며, 그 축(bornRound)은 다음 스코프다.
  const issues = $derived(items.filter((item) => item.kind === 'issue'));
  const lostCards = $derived(issues.filter((item) => !item.anchored));
  const openCards = $derived(issues.filter((item) => item.anchored));
  const settledCards = $derived<MarginItem[]>([]);

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

  const activate = (id: string | null, from: MarginActivationSource = 'manuscript') => {
    activeId = id;
    const current = editor;
    const target = items.find((item) => item.id === id);
    if (!current || target === undefined || !target.anchored) return;
    // 죽은 id로는 아무 데도 가지 못한다 — 코어가 아직 들고 있는 자리로만 데려간다
    const live = new Set(current.appliedSnapshot.trackedRanges.map((range) => range.id));
    const seat = target.rangeIds.find((rangeId) => live.has(rangeId));
    if (seat === undefined) return;

    // 원고에서 출발한 활성은 반대편(카드)만 데려간다 — 원고는 이미 눈앞에 있고, 배치에 밀려 화면 밖으로
    // 나간 카드가 데려올 대상이다. 컬럼이 서지 않는 모드·강점은 카드가 없어 원고 쪽으로 떨어진다.
    if (from === 'manuscript' && mode === 'column' && target.kind === 'issue') {
      const itemId = target.id;
      void tick().then(() => {
        const card = document.querySelector(`[data-prism-card="${itemId}"]`);
        if (card) card.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
        else void current.revealTrackedItem(seat);
      });
      return;
    }

    void current.revealTrackedItem(seat);
  };

  $effect(() => {
    void selectedRoundId;
    untrack(() => {
      activeId = null;
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
    if (builtRoundId !== target.roundId || items.every((item) => item.id !== target.itemId)) return;

    untrack(() => {
      activate(target.itemId, 'jump');
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
      Toast.error(message);
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
    get activeId() {
      return activeId;
    },
    get mode() {
      return mode;
    },
    get ready() {
      return ready;
    },
    get myId() {
      return myId;
    },
    select,
    activate,
    setSegment,
    reply: (threadId, body) => guard(() => replyMutation({ input: { threadId, body } }), '답글을 남기지 못했어요'),
    editComment: (commentId, body) => guard(() => updateCommentMutation({ input: { commentId, body } }), '처리하지 못했어요'),
    deleteComment: (commentId) => guard(() => deleteCommentMutation({ input: { commentId } }), '처리하지 못했어요'),
    close: (threadId) => guard(() => closeMutation({ input: { threadId } }), '처리하지 못했어요'),
    reopen: (threadId) => guard(() => reopenMutation({ input: { threadId } }), '처리하지 못했어요'),
    react: (threadId, value) => guard(() => reactMutation({ input: { threadId, value } }), '처리하지 못했어요'),
  });

  // 리뷰가 없는 문서에까지 여백을 잡아 두면 본문이 통째로 밀린다 — 회차가 걸린 뒤에만 자리를 낸다.
  // 짚은 곳이 하나도 없는 회차도 컬럼은 선다: 자리가 아예 없으면 "짚은 곳이 없어요"를 말할 자리도 없다.
  // 한 번 낸 자리는 회차가 걸려 있는 동안 유지한다: 회차를 갈아탈 때마다 접었다 펴면 본문이 좌우로 두 번 튄다.
  let reserved = $state(false);
  $effect(() => {
    if (selectedRoundId === null) reserved = false;
    else if (ready) reserved = true;
  });

  const insets = $derived(
    mode === 'column' && !idle && reserved ? { left: GUTTER, right: COLUMN_WIDTH + COLUMN_GAP } : { left: 0, right: 0 },
  );

  onDestroy(() => {
    clearTimeout(lostRetry);
    editor?.clearPrismReviewRanges();
  });
</script>

{@render children(insets)}
