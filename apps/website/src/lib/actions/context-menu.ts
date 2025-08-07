import { mount, unmount } from 'svelte';
import Menu from '$lib/components/Menu.svelte';
import type { Snippet } from 'svelte';

export function contextMenu(node: HTMLElement, params: { content: Snippet }) {
  let mountedComponent: ReturnType<typeof mount> | null = null;

  const handleContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();

    if (mountedComponent) {
      unmount(mountedComponent);
      mountedComponent = null;
      delete node.dataset.contextMenuOpen;
    }

    node.dataset.contextMenuOpen = 'true';

    mountedComponent = mount(Menu, {
      target: document.body,
      props: {
        open: true,
        contextMenuPosition: { x: e.clientX, y: e.clientY },
        placement: 'bottom-start',
        offset: 0,
        children: params.content,
        onclose: () => {
          delete node.dataset.contextMenuOpen;
        },
      },
    });
  };

  node.addEventListener('contextmenu', handleContextMenu);

  return {
    destroy() {
      if (mountedComponent) {
        unmount(mountedComponent);
        mountedComponent = null;
        delete node.dataset.contextMenuOpen;
      }
      node.removeEventListener('contextmenu', handleContextMenu);
    },
    update(newParams: { content: Snippet }) {
      params = newParams;
    },
  };
}
