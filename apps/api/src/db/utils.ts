export const first = <T>(arr: T[]): T | undefined => arr[0];
export const firstOrEmptyObject = <T>(arr: T[]): Partial<T> => arr[0] ?? {};
export const firstOrThrow = <T>(arr: T[]): T => {
  if (arr.length === 0) {
    throw new Error('firstOrThrow assertion failed');
  }

  return arr[0];
};

export const firstOrThrowWith =
  (error: Error) =>
  <T>(arr: T[]): T => {
    if (arr.length === 0) {
      throw error;
    }

    return arr[0];
  };
