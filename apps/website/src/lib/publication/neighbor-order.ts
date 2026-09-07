export type NeighborOrders = { lowerOrder: string | null; upperOrder: string | null };

export const appendOrder = (orders: readonly string[]): NeighborOrders => ({ lowerOrder: orders.at(-1) ?? null, upperOrder: null });
