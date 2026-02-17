import AlignCenterIcon from '~icons/lucide/align-center';
import AlignJustifyIcon from '~icons/lucide/align-justify';
import AlignLeftIcon from '~icons/lucide/align-left';
import AlignRightIcon from '~icons/lucide/align-right';

export const values = {
  fontWeight: [
    { label: '가장 가늘게', value: 100 },
    { label: '아주 가늘게', value: 200 },
    { label: '가늘게', value: 300 },
    { label: '보통', value: 400 },
    { label: '중간', value: 500 },
    { label: '약간 굵게', value: 600 },
    { label: '굵게', value: 700 },
    { label: '아주 굵게', value: 800 },
    { label: '가장 굵게', value: 900 },
  ],

  fontSize: [
    { label: '8', value: 8 },
    { label: '9', value: 9 },
    { label: '10', value: 10 },
    { label: '11', value: 11 },
    { label: '12', value: 12 },
    { label: '14', value: 14 },
    { label: '16', value: 16 },
    { label: '18', value: 18 },
    { label: '20', value: 20 },
    { label: '22', value: 22 },
    { label: '24', value: 24 },
    { label: '30', value: 30 },
    { label: '36', value: 36 },
    { label: '48', value: 48 },
    { label: '60', value: 60 },
    { label: '72', value: 72 },
    { label: '96', value: 96 },
  ],

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

  paragraphIndent: [
    { label: '없음', value: 0 },
    { label: '0.5칸', value: 0.5 },
    { label: '1칸', value: 1 },
    { label: '2칸', value: 2 },
  ],

  maxWidth: [
    { label: '400px', value: 400 },
    { label: '600px', value: 600 },
    { label: '800px', value: 800 },
  ],

  blockGap: [
    { label: '없음', value: 0 },
    { label: '0.5줄', value: 0.5 },
    { label: '1줄', value: 1 },
    { label: '2줄', value: 2 },
  ],

  pageLayout: [
    { label: 'A4 (210mm × 297mm)', value: 'a4', width: 210, height: 297, margin: { top: 25, bottom: 25, left: 25, right: 25 } },
    { label: 'A5 (148mm × 210mm)', value: 'a5', width: 148, height: 210, margin: { top: 20, bottom: 20, left: 20, right: 20 } },
    { label: 'B5 (176mm × 250mm)', value: 'b5', width: 176, height: 250, margin: { top: 15, bottom: 15, left: 15, right: 15 } },
    { label: 'B6 (125mm × 176mm)', value: 'b6', width: 125, height: 176, margin: { top: 10, bottom: 10, left: 10, right: 10 } },
  ],
} as const;
