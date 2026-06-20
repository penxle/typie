type EscapeHandler = () => boolean;

const stack: EscapeHandler[] = [];

export function runEscapeStack(): boolean {
  for (let i = stack.length - 1; i >= 0; i--) {
    if (stack[i]()) {
      return true;
    }
  }

  return false;
}

function handleGlobalEscape(e: KeyboardEvent) {
  if (!(e.key === 'Escape' && runEscapeStack())) {
    return;
  }

  e.preventDefault();
  e.stopPropagation();
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
