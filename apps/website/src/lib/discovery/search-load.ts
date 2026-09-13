import { isHttpError } from '@sveltejs/kit';

export const shouldDegradeSearchError = (err: unknown): boolean => isHttpError(err) && err.status === 500;
