import BarChart3Icon from '~icons/lucide/bar-chart-3';
import BookOpenIcon from '~icons/lucide/book-open';
import BookTextIcon from '~icons/lucide/book-text';
import ListTreeIcon from '~icons/lucide/list-tree';
import StickyNoteIcon from '~icons/lucide/sticky-note';
import TargetIcon from '~icons/lucide/target';
import { reviewStartChip } from './review/start-chip.ts';
import type { Component } from 'svelte';

export type StartChip = { label: string; insert: string; icon: Component };

const openDocumentChip: StartChip = {
  label: '열린 문서 훑어보기',
  insert: '지금 열어 둔 문서를 읽고 어떤 글인지 정리해 주세요.',
  icon: BookOpenIcon,
};

const rubyChip: StartChip = {
  label: '일본 라이트노벨풍 루비 달기',
  insert: '지금 열어 둔 문서에 있는 주요 단어들에 일본 라이트노벨풍의 서브컬처스러운 루비를 달아주세요.',
  icon: BookTextIcon,
};

const commonChips: StartChip[] = [
  reviewStartChip,
  { label: '글쓰기 기록 둘러보기', insert: '최근 30일 동안 얼마나 썼는지 알려 주세요.', icon: BarChart3Icon },
  { label: '노트 정리하기', insert: '제 노트들을 훑어보고 정리할 만한 게 있는지 알려 주세요.', icon: StickyNoteIcon },
  {
    label: '스페이스 정리 제안 받기',
    insert: '스페이스를 보고 폴더나 아이콘 등을 정리하면 좋을 점을 제안해 주세요.',
    icon: ListTreeIcon,
  },
  { label: '목표 세우기', insert: '하루 글쓰기 목표를 같이 정해 주세요.', icon: TargetIcon },
];

export const startChipsFor = (hasOpenDocument: boolean): StartChip[] =>
  hasOpenDocument ? [commonChips[0], openDocumentChip, ...commonChips.slice(1), rubyChip] : commonChips;
