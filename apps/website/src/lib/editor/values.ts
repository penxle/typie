import AlignCenterIcon from '~icons/lucide/align-center';
import AlignJustifyIcon from '~icons/lucide/align-justify';
import AlignLeftIcon from '~icons/lucide/align-left';
import AlignRightIcon from '~icons/lucide/align-right';

export const values = {
  fontWeight: {
    100: '가장 가늘게',
    200: '아주 가늘게',
    300: '가늘게',
    400: '보통',
    500: '중간',
    600: '약간 굵게',
    700: '굵게',
    800: '아주 굵게',
    900: '가장 굵게',
  } as Record<number, string>,

  fontSize: [8, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 30, 36, 48, 60, 72, 96] as const,
  minFontSize: 1,
  maxFontSize: 200,

  textColor: [
    { label: '블랙', value: 'black', themeKey: 'text.black' },
    { label: '다크 그레이', value: 'darkgray', themeKey: 'text.darkgray' },
    { label: '그레이', value: 'gray', themeKey: 'text.gray' },
    { label: '라이트 그레이', value: 'lightgray', themeKey: 'text.lightgray' },
    { label: '화이트', value: 'white', themeKey: 'text.white' },
    { label: '레드', value: 'red', themeKey: 'text.red' },
    { label: '오렌지', value: 'orange', themeKey: 'text.orange' },
    { label: '앰버', value: 'amber', themeKey: 'text.amber' },
    { label: '옐로', value: 'yellow', themeKey: 'text.yellow' },
    { label: '라임', value: 'lime', themeKey: 'text.lime' },
    { label: '그린', value: 'green', themeKey: 'text.green' },
    { label: '에메랄드', value: 'emerald', themeKey: 'text.emerald' },
    { label: '틸', value: 'teal', themeKey: 'text.teal' },
    { label: '시안', value: 'cyan', themeKey: 'text.cyan' },
    { label: '스카이', value: 'sky', themeKey: 'text.sky' },
    { label: '블루', value: 'blue', themeKey: 'text.blue' },
    { label: '인디고', value: 'indigo', themeKey: 'text.indigo' },
    { label: '바이올렛', value: 'violet', themeKey: 'text.violet' },
    { label: '퍼플', value: 'purple', themeKey: 'text.purple' },
    { label: '마젠타', value: 'fuchsia', themeKey: 'text.fuchsia' },
    { label: '핑크', value: 'pink', themeKey: 'text.pink' },
    { label: '로즈', value: 'rose', themeKey: 'text.rose' },
  ] as const,

  textBackgroundColor: [
    { label: '배경 없음', value: 'none', themeKey: null },
    { label: '그레이', value: 'gray', themeKey: 'bg.gray' },
    { label: '레드', value: 'red', themeKey: 'bg.red' },
    { label: '오렌지', value: 'orange', themeKey: 'bg.orange' },
    { label: '옐로', value: 'yellow', themeKey: 'bg.yellow' },
    { label: '그린', value: 'green', themeKey: 'bg.green' },
    { label: '블루', value: 'blue', themeKey: 'bg.blue' },
    { label: '퍼플', value: 'purple', themeKey: 'bg.purple' },
  ] as const,

  lineHeight: [
    { label: '80%', value: 0.8 },
    { label: '100%', value: 1 },
    { label: '120%', value: 1.2 },
    { label: '140%', value: 1.4 },
    { label: '160%', value: 1.6 },
    { label: '180%', value: 1.8 },
    { label: '200%', value: 2 },
    { label: '220%', value: 2.2 },
  ] as const,

  letterSpacing: [
    { label: '-10%', value: -0.1 },
    { label: '-5%', value: -0.05 },
    { label: '0%', value: 0 },
    { label: '5%', value: 0.05 },
    { label: '10%', value: 0.1 },
    { label: '20%', value: 0.2 },
    { label: '40%', value: 0.4 },
  ] as const,

  textAlign: [
    { label: '왼쪽 정렬', value: 'left', icon: AlignLeftIcon },
    { label: '가운데 정렬', value: 'center', icon: AlignCenterIcon },
    { label: '오른쪽 정렬', value: 'right', icon: AlignRightIcon },
    { label: '양쪽 정렬', value: 'justify', icon: AlignJustifyIcon },
  ] as const,
} as const;
