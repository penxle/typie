import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = ({ request }) => ({
  desktop: /\bTypie\//.test(request.headers.get('user-agent') ?? ''),
});
