export const values = {
  backgroundColor: [
    { label: '흰색', value: 'white', hex: '#ffffff' },
    { label: '회색', value: 'gray', hex: '#f3f4f6' },
    { label: '노란색', value: 'yellow', hex: '#fef3c7' },
    { label: '파란색', value: 'blue', hex: '#dbeafe' },
    { label: '초록색', value: 'green', hex: '#dcfce7' },
    { label: '분홍색', value: 'pink', hex: '#fce7f3' },
    { label: '주황색', value: 'orange', hex: '#fed7aa' },
    { label: '보라색', value: 'purple', hex: '#e9d5ff' },
  ],

  backgroundStyle: [
    { label: '채우기', value: 'solid' },
    { label: '빗금', value: 'hachure' },
    { label: '없음', value: 'none' },
  ],

  roughness: [
    { label: '매끄럽게', value: 'none' },
    { label: '손그림', value: 'rough' },
  ],

  borderRadius: [
    { label: '직각', value: 'none' },
    { label: '둥글게', value: 'round' },
  ],

  fontSize: [
    { label: '12px', value: 12 },
    { label: '14px', value: 14 },
    { label: '16px', value: 16 },
    { label: '18px', value: 18 },
    { label: '20px', value: 20 },
    { label: '24px', value: 24 },
    { label: '28px', value: 28 },
    { label: '32px', value: 32 },
  ],

  fontFamily: [
    { label: '손글씨', value: 'handwriting', fontFamily: 'GWEduSaeeum' },
    { label: '고딕', value: 'sans-serif', fontFamily: 'Pretendard' },
  ],
} as const;

export const defaultValues = {
  backgroundColor: 'yellow',
  backgroundStyle: 'solid',
  roughness: 'rough',
  borderRadius: 'round',
  fontSize: 16,
  fontFamily: 'handwriting',
} as const;

export type BackgroundColor = (typeof values.backgroundColor)[number]['value'];
export type BackgroundStyle = (typeof values.backgroundStyle)[number]['value'];
export type Roughness = (typeof values.roughness)[number]['value'];
export type BorderRadius = (typeof values.borderRadius)[number]['value'];
export type FontSize = (typeof values.fontSize)[number]['value'];
export type FontFamily = (typeof values.fontFamily)[number]['value'];
