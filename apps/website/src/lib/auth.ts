import { redirect } from '@sveltejs/kit';
import { browser } from '$app/environment';
import { leavePage } from '$lib/navigation';

export async function redirectForAuthentication(url: string): Promise<never> {
  if (browser) {
    await leavePage(url, 'login');
    // The load must not replace the retained dashboard with a redirect/error
    // while the browser or native shell is completing its authenticated teardown.
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- a redirecting load must never publish another result
    return new Promise<never>(() => {});
  }
  redirect(302, url);
}
