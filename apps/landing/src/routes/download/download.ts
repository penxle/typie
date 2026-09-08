import { APP_STORE_URL, DESKTOP_MAC_ARM64_URL, DESKTOP_MAC_X64_URL, DESKTOP_WIN_X64_URL, PLAY_STORE_URL } from '@typie/lib/const';
import GlobeIcon from '~icons/lucide/globe';
import AppleIcon from '~icons/simple-icons/apple';
import AppStoreIcon from '~icons/simple-icons/appstore';
import GooglePlayIcon from '~icons/simple-icons/googleplay';
import WindowsIcon from '~icons/simple-icons/windows';
import { siteUrl } from '$lib/site';
import type { Component } from 'svelte';
import type { PlatformId } from '$lib/platform';

export type Download = { id: string; icon: Component; platform: string; detail: string; url: string; external: boolean };

export type Group = { title: string; note: string; items: readonly Download[] };

export const COPY = {
  pageTitle: '다운로드',
  description: '타이피 데스크톱 앱과 모바일 앱을 내려받으세요.',
  title: '타이피 다운로드',
  sub: '데스크톱과 모바일, 어디서든 이어 쓰세요.',
} as const;

export const DESKTOP: readonly Download[] = [
  { id: 'mac-arm64', icon: AppleIcon, platform: 'macOS', detail: 'Apple Silicon · .dmg', url: DESKTOP_MAC_ARM64_URL, external: false },
  { id: 'mac-x64', icon: AppleIcon, platform: 'macOS', detail: 'Intel · .dmg', url: DESKTOP_MAC_X64_URL, external: false },
  { id: 'win-x64', icon: WindowsIcon, platform: 'Windows', detail: 'x64 · .exe', url: DESKTOP_WIN_X64_URL, external: false },
];

export const MOBILE: readonly Download[] = [
  { id: 'ios', icon: AppStoreIcon, platform: 'iOS', detail: 'App Store', url: APP_STORE_URL, external: true },
  { id: 'android', icon: GooglePlayIcon, platform: 'Android', detail: 'Google Play', url: PLAY_STORE_URL, external: true },
];

export const WEB: Download = { id: 'web', icon: GlobeIcon, platform: '웹', detail: 'typie.co', url: siteUrl('/start'), external: false };

export const DESKTOP_GROUP: Group = { title: '데스크톱', note: 'macOS 11 이상 · Windows 10 이상', items: DESKTOP };
export const MOBILE_GROUP: Group = { title: '모바일', note: 'iOS · Android', items: MOBILE };
export const WEB_GROUP: Group = { title: '웹', note: '', items: [WEB] };

export type Primary = { item: Download; label: string; note: string };

export const PRIMARY: Record<Exclude<PlatformId, 'web'>, Primary> = {
  mac: { item: DESKTOP[0], label: 'macOS용 다운로드', note: 'macOS 11 이상 · Apple Silicon' },
  windows: { item: DESKTOP[2], label: 'Windows용 다운로드', note: 'Windows 10 이상 · x64' },
  ios: { item: MOBILE[0], label: 'App Store에서 받기', note: 'iPhone · iPad' },
  android: { item: MOBILE[1], label: 'Google Play에서 받기', note: 'Android' },
};

export const primaryFor = (id: PlatformId | null): Primary | null => (id && id !== 'web' ? PRIMARY[id] : null);
