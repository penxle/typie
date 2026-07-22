import { error } from '@sveltejs/kit';
import { desc } from 'drizzle-orm';
import { createDb, PromptVariants } from '$lib/server/db/index.ts';
import { loadVariantStatusIndex } from '../lib/status.ts';
import type { PageServerLoad } from './$types';

type VariantRow = { id: string; label: string; note: string | null; baseVariantId: string | null; createdAt: Date };

// baseVariantId 계보를 따라 트리를 구성하고 depth-first로 평탄화한다 (목록에서 들여쓰기로 표시하기 위함).
const flattenLineage = (variants: VariantRow[]): { variant: VariantRow; depth: number }[] => {
  const byId = new Map(variants.map((v) => [v.id, v]));
  const childrenByBase = new Map<string, VariantRow[]>();
  const roots: VariantRow[] = [];

  for (const variant of variants) {
    if (variant.baseVariantId && byId.has(variant.baseVariantId)) {
      const siblings = childrenByBase.get(variant.baseVariantId) ?? [];
      siblings.push(variant);
      childrenByBase.set(variant.baseVariantId, siblings);
    } else {
      roots.push(variant);
    }
  }

  for (const siblings of childrenByBase.values()) {
    siblings.sort((a, b) => a.createdAt.getTime() - b.createdAt.getTime());
  }
  roots.sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime());

  const result: { variant: VariantRow; depth: number }[] = [];
  const visit = (variant: VariantRow, depth: number) => {
    result.push({ variant, depth });
    for (const child of childrenByBase.get(variant.id) ?? []) {
      visit(child, depth + 1);
    }
  };
  for (const root of roots) visit(root, 0);

  return result;
};

export const load: PageServerLoad = async ({ platform }) => {
  if (!platform) {
    error(500, 'platform unavailable');
  }

  const db = createDb(platform.env.DB);

  // content 컬럼은 목록에서 쓰지 않으므로(계보/상태/note만 필요) 오버페치를 피해 필요한 컬럼만 선택한다.
  const [variants, statusIndex] = await Promise.all([
    db
      .select({
        id: PromptVariants.id,
        label: PromptVariants.label,
        note: PromptVariants.note,
        baseVariantId: PromptVariants.baseVariantId,
        createdAt: PromptVariants.createdAt,
      })
      .from(PromptVariants)
      .orderBy(desc(PromptVariants.createdAt)),
    loadVariantStatusIndex(db),
  ]);

  const lineage = flattenLineage(variants).map(({ variant, depth }) => ({
    id: variant.id,
    label: variant.label,
    note: variant.note,
    depth,
    status: statusIndex.deriveStatus(variant.id),
    createdAt: variant.createdAt.toISOString(),
  }));

  return { lineage };
};
