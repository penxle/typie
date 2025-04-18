import postgres from 'postgres';

export const first = <T>(arr: T[]): T | undefined => arr[0];
export const firstOrThrow = <T>(arr: T[]): T => {
  if (arr.length === 0) {
    throw new Error('firstOrThrow assertion failed');
  }

  return arr[0];
};

export const catchUniqueViolationAndThrow = (errorThrower: () => Error) => {
  return (err: unknown) => {
    // 23505 = PostgreSQL unique violation error code
    if (err instanceof postgres.PostgresError && err.code === '23505') {
      throw errorThrower();
    }

    throw err;
  };
};
