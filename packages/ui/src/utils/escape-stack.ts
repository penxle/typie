type EscapeHandler = () => boolean;

const stack: EscapeHandler[] = [];

function handleGlobalEscape(e: KeyboardEvent) {
  if (e.key === 'Escape' && stack.length > 0) {
    // LIFO
    for (let i = stack.length - 1; i >= 0; i--) {
      const handler = stack[i];
      const handled = handler();

      if (handled) {
        e.preventDefault();
        e.stopPropagation();
        return;
      }
    }
  }
}

if (typeof window !== 'undefined') {
  window.addEventListener('keydown', handleGlobalEscape);
}

export function pushEscapeHandler(handler: EscapeHandler) {
  stack.push(handler);

  return () => {
    const index = stack.indexOf(handler);
    if (index !== -1) {
      stack.splice(index, 1);
    }
  };
}
