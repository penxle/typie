import { error } from '@sveltejs/kit';

export const parseJsonBody = async (request: Request): Promise<unknown> => {
  try {
    return await request.json();
  } catch {
    error(400, 'invalid JSON body');
  }
};
