<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Checkbox } from '@typie/ui/components';
  import { EntityVisibility } from '@/enums';
  import { getTreeContext } from '../state.svelte';
  import type { TreeEntity } from './types';

  type Props = {
    entityId: string;
    visibility?: EntityVisibility;
  };

  let { entityId, visibility }: Props = $props();

  const treeState = getTreeContext();
  const selected = $derived(treeState.selectedEntityIds.has(entityId));

  const someDescendants = (entityId: string, someFn: (entity: TreeEntity) => boolean) => {
    for (const child of treeState.entityMap.get(entityId)?.children ?? []) {
      if (someFn(child)) {
        return true;
      }

      if (someDescendants(child.id, someFn)) {
        return true;
      }
    }

    return false;
  };

  const descendants = (entityId: string, fn: (entity: TreeEntity) => void) => {
    for (const child of treeState.entityMap.get(entityId)?.children ?? []) {
      fn(child);
      descendants(child.id, fn);
    }
  };

  const select = (entityId: string, origin = true) => {
    treeState.selectedEntityIds.add(entityId);

    if (origin) {
      treeState.lastSelectedEntityId = entityId;
    }

    descendants(entityId, (entity) => {
      select(entity.id, false);
    });
  };

  const deselect = (entityId: string) => {
    treeState.selectedEntityIds.delete(entityId);

    // NOTE: 모든 자손이 선택되어 있지 않으면 모든 자손을 선택 해제
    if (
      !someDescendants(entityId, (entity) => {
        return !treeState.selectedEntityIds.has(entity.id);
      })
    ) {
      descendants(entityId, (entity) => {
        treeState.selectedEntityIds.delete(entity.id);
      });
    }

    // NOTE: 모든 부모를 선택 해제
    let parentId = treeState.entityMap.get(entityId)?.parentId;
    while (parentId) {
      treeState.selectedEntityIds.delete(parentId);
      parentId = treeState.entityMap.get(parentId)?.parentId;
    }
  };

  const getAllEntityIds = () => {
    const ids: string[] = [];
    const tree = treeState.element;
    const collectIds = (entities: TreeEntity[]) => {
      entities.forEach((entity) => {
        ids.push(entity.id);

        if (entity.children && tree) {
          const folderElement = tree.querySelector(`[data-id="${entity.id}"]`) as HTMLDetailsElement;
          const isOpen = folderElement?.open ?? false;

          if (isOpen) {
            collectIds(entity.children);
          }
        }
      });
    };
    collectIds(treeState.entities);
    return ids;
  };

  const selectEntityRange = () => {
    const fromId = treeState.lastSelectedEntityId ?? entityId;
    const toId = entityId;
    const allIds = getAllEntityIds();

    const fromIndex = allIds.indexOf(fromId);
    const toIndex = allIds.indexOf(toId);

    if (fromIndex === -1 || toIndex === -1) return;

    const startIndex = Math.min(fromIndex, toIndex);
    const endIndex = Math.max(fromIndex, toIndex);

    for (let i = startIndex; i <= endIndex; i++) {
      select(allIds[i], false);
    }
    treeState.lastSelectedEntityId = toId;
  };

  const handleToggle = (e: MouseEvent) => {
    e.stopPropagation();
    if (e.shiftKey) {
      if (selected) {
        e.preventDefault();
      }
      selectEntityRange();
    } else {
      if (selected) {
        deselect(entityId);
      } else {
        select(entityId);
      }
    }
  };
</script>

<div class={css({ position: 'relative', flex: 'none', size: '16px' })}>
  <div
    class={css(
      {
        position: 'absolute',
        inset: '0',
        borderRadius: 'full',
        backgroundColor: 'interactive.hover',
        size: '4px',
        margin: 'auto',
        opacity: '100',
        transition: 'common',
        _groupHover: { opacity: '0' },
      },
      visibility === EntityVisibility.UNLISTED && { backgroundColor: 'accent.brand.default' },
      selected && { opacity: '0' },
    )}
  ></div>
  <div
    class={css(
      {
        position: 'absolute',
        inset: '0',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        opacity: '0',
        transition: 'common',
        _groupHover: { opacity: '100' },
      },
      selected && { opacity: '100' },
    )}
  >
    <Checkbox checked={selected} clickPadding={true} onclick={handleToggle} size="sm" />
  </div>
</div>
