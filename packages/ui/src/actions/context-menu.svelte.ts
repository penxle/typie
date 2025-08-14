import { getAllContexts, mount, unmount } from 'svelte';
import { Menu } from '../components';
import type { Snippet } from 'svelte';

export function contextMenu(node: HTMLElement, params: { content: Snippet }) {
  $effect(() => {
    let mountedComponent: ReturnType<typeof mount> | null = null;
    const context = getAllContexts();

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
        context,
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

    return () => {
      if (mountedComponent) {
        unmount(mountedComponent);
        mountedComponent = null;
        delete node.dataset.contextMenuOpen;
      }
      node.removeEventListener('contextmenu', handleContextMenu);
    };
  });
}
