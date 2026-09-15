export const titleSlot = (titleBottom: number | undefined, stickyBottom: number): '' | 'title' =>
  titleBottom !== undefined && titleBottom <= stickyBottom ? 'title' : '';

export const readingProgress = (scrollY: number, scrollHeight: number, innerHeight: number): number => {
  const max = scrollHeight - innerHeight;
  return max > 0 ? Math.min(1, Math.max(0, scrollY / max)) : 0;
};

export const chromeHidden = (input: { post: boolean; retreat: boolean; desktop: boolean }): boolean =>
  input.post && input.retreat && !input.desktop;
