import { APP_STORE_URL, DESKTOP_MAC_ARM64_URL, DESKTOP_WIN_X64_URL, PLAY_STORE_URL } from '@typie/lib/const';
import GlobeIcon from '~icons/lucide/globe';
import AppleIcon from '~icons/simple-icons/apple';
import AppStoreIcon from '~icons/simple-icons/appstore';
import GooglePlayIcon from '~icons/simple-icons/googleplay';
import WindowsIcon from '~icons/simple-icons/windows';
import type { Component } from 'svelte';

export type PlatformId = 'web' | 'mac' | 'windows' | 'ios' | 'android';

export type Platform = { id: PlatformId; name: string; note?: string; url: string; icon: Component; external: boolean };

export const PLATFORMS: readonly Platform[] = [
  { id: 'web', name: '웹', url: '/start', icon: GlobeIcon, external: false },
  {
    id: 'mac',
    name: 'macOS',
    note: 'Apple Silicon',
    url: DESKTOP_MAC_ARM64_URL,
    icon: AppleIcon,
    external: false,
  },
  {
    id: 'windows',
    name: 'Windows',
    note: 'x64',
    url: DESKTOP_WIN_X64_URL,
    icon: WindowsIcon,
    external: false,
  },
  { id: 'ios', name: 'iOS', note: 'App Store', url: APP_STORE_URL, icon: AppStoreIcon, external: true },
  {
    id: 'android',
    name: 'Android',
    note: 'Play Store',
    url: PLAY_STORE_URL,
    icon: GooglePlayIcon,
    external: true,
  },
];

export const DOWNLOADS = PLATFORMS.filter((platform) => platform.id !== 'web');

export const detectPlatform = (): PlatformId | null => {
  if (typeof navigator === 'undefined') return null;
  const ua = navigator.userAgent;
  if (/iPhone|iPad|iPod/.test(ua) || (/Macintosh/.test(ua) && navigator.maxTouchPoints > 1)) return 'ios';
  if (/Android/.test(ua)) return 'android';
  if (/Macintosh/.test(ua)) return 'mac';
  if (/Windows/.test(ua)) return 'windows';
  return null;
};
