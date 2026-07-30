export function resolveNextFractionalOrderMove<K>(
  authoritativeOrders: ReadonlyMap<K, string>,
  desiredKeys: readonly K[],
  preferredKey?: K,
): { key: K; lowerOrder?: string; upperOrder?: string } | null {
  const desiredKeySet = new Set(desiredKeys);
  if (authoritativeOrders.size !== desiredKeys.length || desiredKeySet.size !== desiredKeys.length) {
    return null;
  }
  for (const key of authoritativeOrders.keys()) {
    if (!desiredKeySet.has(key)) return null;
  }

  const authoritativeKeys = [...authoritativeOrders].toSorted((left, right) => left[1].localeCompare(right[1])).map(([key]) => key);
  const mismatchIndex = authoritativeKeys.findIndex((key, index) => key !== desiredKeys[index]);
  if (mismatchIndex === -1) return null;

  let key = desiredKeys[mismatchIndex];
  let useDesiredNeighbors = false;
  if (preferredKey !== undefined && desiredKeySet.has(preferredKey)) {
    const authoritativeWithoutPreferred = authoritativeKeys.filter((authoritativeKey) => authoritativeKey !== preferredKey);
    const desiredWithoutPreferred = desiredKeys.filter((desiredKey) => desiredKey !== preferredKey);
    if (
      authoritativeWithoutPreferred.length === desiredWithoutPreferred.length &&
      authoritativeWithoutPreferred.every((authoritativeKey, index) => authoritativeKey === desiredWithoutPreferred[index])
    ) {
      key = preferredKey;
      useDesiredNeighbors = true;
    }
  }
  if (key === undefined) return null;
  const withoutMovedKey = authoritativeKeys.filter((authoritativeKey) => authoritativeKey !== key);
  const desiredIndex = desiredKeys.indexOf(key);
  const lowerKey = useDesiredNeighbors ? desiredKeys[desiredIndex - 1] : withoutMovedKey[mismatchIndex - 1];
  const upperKey = useDesiredNeighbors ? desiredKeys[desiredIndex + 1] : withoutMovedKey[mismatchIndex];
  const lowerOrder = lowerKey === undefined ? undefined : authoritativeOrders.get(lowerKey);
  const upperOrder = upperKey === undefined ? undefined : authoritativeOrders.get(upperKey);
  if (lowerOrder !== undefined && upperOrder !== undefined && lowerOrder >= upperOrder) return null;

  return { key, lowerOrder, upperOrder };
}
