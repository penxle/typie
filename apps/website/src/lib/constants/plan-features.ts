import TypeIcon from '~icons/lucide/book-open-text';
import EllipsisIcon from '~icons/lucide/ellipsis';
import FlaskConicalIcon from '~icons/lucide/flask-conical';
import HeadsetIcon from '~icons/lucide/headset';
import ImagesIcon from '~icons/lucide/images';
import LinkIcon from '~icons/lucide/link';
import SpellCheckIcon from '~icons/lucide/spell-check';
import SproutIcon from '~icons/lucide/sprout';
import TypeOutlineIcon from '~icons/lucide/type-outline';
import type { Component } from 'svelte';

export type PlanFeature = {
  icon: Component;
  label: string;
};

export const PLAN_FEATURES: Record<'basic' | 'full', PlanFeature[]> = {
  basic: [
    { icon: TypeIcon, label: '총 16,000자의 글자 작성' },
    { icon: ImagesIcon, label: '총 20MB의 파일 업로드' },
  ],
  full: [
    { icon: TypeIcon, label: '무제한 글자 수' },
    { icon: ImagesIcon, label: '무제한 파일 업로드' },
    { icon: SpellCheckIcon, label: '맞춤법 검사' },
    { icon: LinkIcon, label: '커스텀 게시 주소' },
    { icon: TypeOutlineIcon, label: '커스텀 폰트 업로드' },
    { icon: FlaskConicalIcon, label: '베타 기능 우선 접근' },
    { icon: HeadsetIcon, label: '문제 발생 시 우선 지원' },
    { icon: SproutIcon, label: '디스코드 커뮤니티 참여' },
    { icon: EllipsisIcon, label: '그리고 더 많은 혜택' },
  ],
};
