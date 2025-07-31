<script lang="ts">
  import { EntityVisibility } from '@/enums';
  import { Checkbox } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import type { TreeEntity } from './types';

  type Props = {
    entityId: string;
    visibility?: EntityVisibility;
  };

  let { entityId, visibility }: Props = $props();

  let element: HTMLDivElement;

  const app = getAppContext();
  const selected = $derived(app.state.tree.selectedEntityIds.has(entityId));

  const someDescendants = (entityId: string, someFn: (entity: TreeEntity) => boolean) => {
    for (const child of app.state.tree.entityMap.get(entityId)?.children ?? []) {
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
    for (const child of app.state.tree.entityMap.get(entityId)?.children ?? []) {
      fn(child);
      descendants(child.id, fn);
    }
  };

  const select = (entityId: string, origin = true) => {
    app.state.tree.selectedEntityIds.add(entityId);

    if (origin) {
      app.state.tree.lastSelectedEntityId = entityId;
    }

    descendants(entityId, (entity) => {
      select(entity.id, false);
    });
  };

  const deselect = (entityId: string) => {
    app.state.tree.selectedEntityIds.delete(entityId);

    // NOTE: 모든 자손이 선택되어 있지 않으면 모든 자손을 선택 해제
    if (
      !someDescendants(entityId, (entity) => {
        return !app.state.tree.selectedEntityIds.has(entity.id);
      })
    ) {
      descendants(entityId, (entity) => {
        app.state.tree.selectedEntityIds.delete(entity.id);
      });
    }

    // NOTE: 모든 부모를 선택 해제
    let parentId = app.state.tree.entityMap.get(entityId)?.parentId;
    while (parentId) {
      app.state.tree.selectedEntityIds.delete(parentId);
      parentId = app.state.tree.entityMap.get(parentId)?.parentId;
    }
  };

  const getAllEntityIds = () => {
    const ids: string[] = [];
    const tree = element.closest<HTMLElement>('[role="tree"]');
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
    collectIds(app.state.tree.entities);
    return ids;
  };

  const selectEntityRange = () => {
    const fromId = app.state.tree.lastSelectedEntityId ?? entityId;
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
    app.state.tree.lastSelectedEntityId = toId;
  };

  const handleToggle = (e: MouseEvent) => {
    e.stopPropagation();
    if (e.shiftKey) {
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

<div bind:this={element} class={css({ position: 'relative', flex: 'none', size: '16px' })}>
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
