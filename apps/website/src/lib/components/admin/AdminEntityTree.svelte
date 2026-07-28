<script
  generics="T extends {id:string;type:EntityType;state:EntityState;node:{__typename:string;title?:string|null;name?:string|null}}"
  lang="ts"
>
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import FileTextIcon from '~icons/lucide/file-text';
  import FolderIcon from '~icons/lucide/folder';
  import { entityStateLabels, entityStateTones } from '$lib/admin-labels';
  import AdminBadge from './AdminBadge.svelte';
  import type { EntityState, EntityType } from '@typie/lib/enums';
  import type { EntityTreeNode } from '$lib/admin-entity-tree';

  type Props = {
    nodes: EntityTreeNode<T>[];
  };

  let { nodes }: Props = $props();

  const labelOf = (entity: T) => entity.node.title ?? entity.node.name ?? '(제목 없음)';
</script>

{#snippet row(node: EntityTreeNode<T>)}
  <div class={flex({ alignItems: 'stretch', borderRadius: '6px', paddingX: '6px', _hover: { backgroundColor: 'admin.card.hover' } })}>
    {#each Array.from({ length: node.depth }, (_, i) => i) as i (i)}
      <span class={css({ width: '20px', flexShrink: '0', borderLeftWidth: '1px', borderColor: 'border.subtle', marginLeft: '7px' })}></span>
    {/each}
    <div class={flex({ alignItems: 'center', gap: '8px', paddingY: '6px', fontSize: '13px', minWidth: '0' })}>
      <Icon icon={node.entity.type === 'FOLDER' ? FolderIcon : FileTextIcon} size={14} />
      <a class={css({ _hover: { textDecoration: 'underline' } })} href="/admin/entities/{node.entity.id}">
        {labelOf(node.entity)}
      </a>
      {#if node.entity.state !== 'ACTIVE'}
        <AdminBadge label={entityStateLabels[node.entity.state]} tone={entityStateTones[node.entity.state]} />
      {/if}
    </div>
  </div>

  {#each node.children as child (child.entity.id)}
    {@render row(child)}
  {/each}
{/snippet}

<div>
  {#each nodes as node (node.entity.id)}
    {@render row(node)}
  {/each}
</div>
