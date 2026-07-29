// feedback_sets.review JSON을 run_items 행으로 전개한다.
//
// SQL로 하지 않는 이유: 세대별 모양 차이(v1.11 이전 strengths가 문자열, cleared 부재)를
// json_each로 다루기엔 취약하고, feedbackIndexes를 항목 id로 푸는 일이 섞여 있다.
// 원본은 migrate-v2.sql이 ledgers의 legacy/review에 백업해 두므로 전개가 틀려도 복구된다.

export type ExpandedItem = {
  kind: string;
  ord: number;
  body: string;
  facets: Record<string, string>;
  anchors: { quoteStart: string; quoteEnd: string; matchStart: number | null; matchEnd: number | null }[];
  // 이미 저장된 지적을 가리키므로 인덱스가 아니라 id다.
  links: string[];
};

const asText = (value: unknown): string => (typeof value === 'string' ? value : '');
const asPosition = (value: unknown): number | null => (Number.isSafeInteger(value) ? (value as number) : null);
const asArray = (value: unknown): Record<string, unknown>[] =>
  Array.isArray(value) ? value.filter((v): v is Record<string, unknown> => !!v && typeof v === 'object') : [];

export const expandReview = (raw: unknown, findingIds: string[]): ExpandedItem[] => {
  if (!raw || typeof raw !== 'object') return [];
  const review = raw as Record<string, unknown>;
  const items: ExpandedItem[] = [];

  const push = (kind: string, body: string, extra: Partial<ExpandedItem> = {}) => {
    if (!body.trim()) return;
    items.push({
      kind,
      ord: items.filter((i) => i.kind === kind).length,
      body: body.trim(),
      facets: {},
      anchors: [],
      links: [],
      ...extra,
    });
  };

  const toLinks = (value: unknown): string[] =>
    (Array.isArray(value) ? value : [])
      .filter((i): i is number => Number.isSafeInteger(i) && i >= 0 && i < findingIds.length)
      .map((i) => findingIds[i]);

  push('characterization', asText(review.characterization));

  // 라운드 3까지의 실행은 강점이 문단 하나였다 — 위치 없는 항목 하나로 읽는다.
  if (typeof review.strengths === 'string') {
    push('strength', review.strengths);
  } else {
    for (const strength of asArray(review.strengths)) {
      push('strength', asText(strength.body), {
        anchors: [
          {
            quoteStart: asText(strength.quoteStart),
            quoteEnd: asText(strength.quoteEnd),
            matchStart: asPosition(strength.matchStart),
            matchEnd: asPosition(strength.matchEnd),
          },
        ],
      });
    }
  }

  for (const entry of asArray(review.cleared)) {
    push('cleared', asText(entry.note), { facets: { axis: asText(entry.axis) } });
  }
  for (const pattern of asArray(review.patterns)) {
    push('pattern', asText(pattern.body), { facets: { theme: asText(pattern.theme) }, links: toLinks(pattern.feedbackIndexes) });
  }
  for (const entry of asArray(review.priority)) {
    push('priority', asText(entry.body), { links: toLinks(entry.feedbackIndexes) });
  }

  return items;
};

// ── 실행 진입점 ──
// wrangler d1 execute --json --file scripts/dump-reviews.sql 의 출력을 표준입력으로 받아
// INSERT 문을 표준출력으로 낸다. wrangler와 결합하지 않아 중간 산출물을 파일로 고정할 수 있고,
// 전개가 틀렸을 때 원본(ledgers.legacy/review)과 생성된 SQL을 나란히 대조할 수 있다.

const quote = (value: string): string => `'${value.replaceAll("'", "''")}'`;

const asJson = (value: unknown): unknown => (typeof value === 'string' ? JSON.parse(value) : value);

export const renderInserts = (rows: { runId: string; review: unknown; findingIds: unknown }[]): string[] => {
  const out: string[] = [];
  for (const row of rows) {
    const findingIds = (asJson(row.findingIds) as string[] | null) ?? [];
    const items = expandReview(asJson(row.review), findingIds);

    for (const item of items) {
      const itemId = crypto.randomUUID();
      out.push(
        `INSERT OR IGNORE INTO run_items (id, run_id, kind, ord, body, facets) VALUES (${quote(itemId)}, ${quote(row.runId)}, ` +
          `${quote(item.kind)}, ${item.ord}, ${quote(item.body)}, ${quote(JSON.stringify(item.facets))});`,
      );
      for (const [ord, anchor] of item.anchors.entries()) {
        out.push(
          `INSERT OR IGNORE INTO item_anchors (id, item_id, ord, start_text, end_text, match_start, match_end, note) VALUES (` +
            `${quote(crypto.randomUUID())}, ${quote(itemId)}, ${ord}, ${quote(anchor.quoteStart)}, ${quote(anchor.quoteEnd)}, ` +
            `${anchor.matchStart ?? 'NULL'}, ${anchor.matchEnd ?? 'NULL'}, NULL);`,
        );
      }
      for (const [ord, target] of item.links.entries()) {
        out.push(`INSERT OR IGNORE INTO item_links (item_id, target_item_id, ord) VALUES (${quote(itemId)}, ${quote(target)}, ${ord});`);
      }
    }
  }
  return out;
};

// wrangler --json은 문장마다 { results, success, meta } 하나를 배열로 낸다.
export const collectRows = (raw: string): { runId: string; review: unknown; findingIds: unknown }[] => {
  const start = raw.indexOf('[');
  if (start === -1) return [];
  const blocks = JSON.parse(raw.slice(start)) as { results?: Record<string, unknown>[] }[];
  return blocks.flatMap((b) => (b.results ?? []).map((r) => ({ runId: String(r.runId), review: r.review, findingIds: r.findingIds })));
};

if (process.argv[1]?.endsWith('expand-reviews.ts')) {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) chunks.push(chunk as Buffer);
  const rows = collectRows(Buffer.concat(chunks).toString('utf8'));
  const statements = renderInserts(rows);
  console.warn(`실행 ${rows.length}건에서 항목 ${statements.filter((s) => s.startsWith('INSERT OR IGNORE INTO run_items')).length}건 전개`);
  process.stdout.write(statements.join('\n') + '\n');
}
