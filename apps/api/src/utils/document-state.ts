export const isSnapshotUsable = <T extends { projectionDegraded: boolean }>(state: T | null | undefined): state is T =>
  state != null && !state.projectionDegraded;
