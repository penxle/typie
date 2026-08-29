const query = '(prefers-reduced-motion: reduce)';
const mediaQuery = globalThis.matchMedia?.(query);
let subscribedMatches = $state(mediaQuery?.matches ?? false);
let receivedChange = false;

mediaQuery?.addEventListener('change', () => {
  receivedChange = true;
  subscribedMatches = mediaQuery.matches;
});

export const prefersReducedMotion = {
  get current(): boolean {
    // Keep synchronous consumers useful when matchMedia becomes available after module evaluation.
    // Once the subscribed query changes, its reactive value remains authoritative.
    const reactiveValue = subscribedMatches;
    return receivedChange ? reactiveValue : (globalThis.matchMedia?.(query).matches ?? reactiveValue);
  },
};
