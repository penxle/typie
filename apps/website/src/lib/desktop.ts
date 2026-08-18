import { browser } from '$app/environment';
import type { TypieDesktopBridge } from '@typie/lib/desktop';

export const desktop: TypieDesktopBridge | null = browser ? (window.typieDesktop ?? null) : null;
