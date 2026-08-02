export type Font = { id: string; weight: number; subfamilyDisplayName?: string | null; url: string; state: string };

export function getRepresentativeFont(fonts: readonly Font[]): Font | null {
  const active = fonts.filter((f) => f.state === 'ACTIVE');
  if (active.length === 0) return null;
  return active.reduce((prev, curr) => {
    const prevDiff = Math.abs(prev.weight - 400);
    const currDiff = Math.abs(curr.weight - 400);
    if (currDiff < prevDiff) return curr;
    if (currDiff === prevDiff && curr.weight > prev.weight) return curr;
    return prev;
  });
}
