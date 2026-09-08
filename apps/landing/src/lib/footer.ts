import GithubIcon from '~icons/simple-icons/github';
import XIcon from '~icons/simple-icons/x';
import type { Component } from 'svelte';

export type FooterLink = { label: string; href: string; external?: boolean };

export const SERVICE_LINKS: readonly FooterLink[] = [
  { label: '소개', href: '/' },
  { label: '구독 안내', href: '/pricing' },
  { label: '업데이트 노트', href: '/changelog' },
  { label: '다운로드', href: '/download' },
  { label: '오픈 대시보드', href: '/open' },
  { label: '고객센터', href: 'https://penxle.channel.io', external: true },
];

export const LEGAL_LINKS: readonly FooterLink[] = [
  { label: '이용약관', href: '/legal/terms' },
  { label: '개인정보처리방침', href: '/legal/privacy' },
];

export const COMPANY_LINES = [
  '(주) 펜슬컴퍼니',
  '대표: 배준현 | 개인정보관리책임자: 배준현',
  '사업자등록번호: 610-88-03078',
  '통신판매업신고번호: 2023-서울강남-4541',
  '서울특별시 강남구 강남대로100길 14, 6층',
  '02-565-7695',
] as const;

export const SOCIAL_LINKS: readonly { label: string; href: string; icon: Component }[] = [
  { label: 'X', href: 'https://x.com/typieofficial', icon: XIcon },
  { label: 'GitHub', href: 'https://github.com/penxle', icon: GithubIcon },
];
