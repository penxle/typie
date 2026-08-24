<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { z } from 'zod';
  import { graphql } from '$mearie';
  import { consequenceClass } from './action-cards.ts';
  import type { EntityVisibility } from '@typie/lib/enums';
  import type { ActionBodyProps } from './action-cards.ts';

  let { input, result, onReady }: ActionBodyProps = $props();

  const VISIBILITY_LABELS: Record<EntityVisibility, string> = {
    PUBLIC: '공개 조회',
    UNLISTED: '링크 조회',
    PRIVATE: '비공개',
  };
  const CONSEQUENCES: Record<EntityVisibility, string> = {
    PUBLIC: '누구나 볼 수 있게 돼요.',
    UNLISTED: '링크가 있는 사람이 볼 수 있게 돼요.',
    PRIVATE: '다른 사람은 더 볼 수 없어요.',
  };

  const Visibility = z.enum(['PUBLIC', 'UNLISTED', 'PRIVATE']);
  const TargetsSchema = z.object({
    ids: z.array(z.string()).min(1).max(20),
    visibility: Visibility,
    recursive: z.boolean().optional(),
  });
  const SnapshotSchema = z.object({
    ok: z.literal(true),
    changes: z.array(
      z.object({ id: z.string(), kind: z.enum(['document', 'folder']), title: z.string().nullable(), from: Visibility, to: Visibility }),
    ),
  });

  type Target = { kind: 'document' | 'folder'; name: string | null; visibility: EntityVisibility };

  const parsed = $derived(TargetsSchema.safeParse(input));
  const ids = $derived(parsed.success ? [...new Set(parsed.data.ids)] : []);
  const visibility = $derived(parsed.success ? parsed.data.visibility : undefined);
  const recursive = $derived(parsed.success && (parsed.data.recursive ?? false));

  const snapshot = $derived(SnapshotSchema.safeParse(result));
  const snapshotTargets = $derived<Target[] | null>(
    snapshot.success ? snapshot.data.changes.map((change) => ({ kind: change.kind, name: change.title, visibility: change.from })) : null,
  );

  const query = createQuery(
    graphql(`
      query DashboardLayout_PrismUpdateSharingBody_Query($ids: [ID!]!) {
        prismEntities(ids: $ids) {
          id
          visibility

          node {
            __typename

            ... on Document {
              id
              title
            }

            ... on Folder {
              id
              name
            }
          }
        }
      }
    `),
    () => ({ ids }),
    () => ({ skip: ids.length === 0 || snapshotTargets !== null }),
  );

  const found = $derived(query.data?.prismEntities ?? []);

  const liveTargets = $derived<(Target | null)[]>(
    ids.map((id) => {
      const entity = found.find((candidate) => candidate.id === id || candidate.node.id === id);
      if (entity === undefined) return null;
      const node = entity.node;
      return node.__typename === 'Folder'
        ? { kind: 'folder', name: node.name, visibility: entity.visibility }
        : { kind: 'document', name: node.title, visibility: entity.visibility };
    }),
  );

  const targets = $derived(snapshotTargets ?? liveTargets);
  const loading = $derived(snapshotTargets === null && ids.length > 0 && query.loading);
  const resolved = $derived(
    snapshotTargets !== null || (ids.length > 0 && !loading && query.error === undefined && targets.every((target) => target !== null)),
  );
  const hasFolder = $derived(resolved && targets.some((target) => target?.kind === 'folder'));

  $effect(() => {
    onReady(resolved);
  });

  const listClass = flex({ flexDirection: 'column', gap: '6px' });
  const rowClass = flex({ alignItems: 'center', gap: '8px' });
  const nameClass = css({
    flex: '1',
    minWidth: '0',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
    lineHeight: '[1.5]',
  });
  const changeClass = flex({ alignItems: 'center', gap: '4px', flexShrink: '0', fontSize: '[11.5px]', color: 'text.faint' });
  const nextClass = css({ fontWeight: 'semibold', color: 'text.subtle' });
  const noticeClass = css({ color: 'text.faint' });
</script>

{#if loading}
  <p class={noticeClass}>대상을 확인하고 있어요</p>
{:else if !resolved || visibility === undefined}
  <p class={noticeClass}>대상을 찾지 못했어요</p>
{:else}
  <div class={listClass}>
    {#each targets as target, index (index)}
      {#if target}
        <div class={rowClass}>
          <p class={nameClass}>{target.kind === 'folder' ? '폴더' : '문서'} 「{target.name ?? '(제목 없음)'}」</p>

          <div class={changeClass}>
            <span>{VISIBILITY_LABELS[target.visibility]}</span>
            <span>→</span>
            <span class={nextClass}>{VISIBILITY_LABELS[visibility]}</span>
          </div>
        </div>
      {/if}
    {/each}
  </div>

  <p class={consequenceClass}>{recursive && hasFolder ? '폴더 안의 것까지 함께 바뀌어요. ' : ''}{CONSEQUENCES[visibility]}</p>
{/if}
