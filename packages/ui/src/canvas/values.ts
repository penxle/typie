export const values = {
  backgroundColor: [
    { label: '흰색', value: 'white', color: '#ffffff', darkColor: '#7f8083' },
    { label: '회색', value: 'gray', color: '#f3f4f6', darkColor: '#6d6e70' },
    { label: '노란색', value: 'yellow', color: '#fef3c7', darkColor: '#9e8e6b' },
    { label: '파란색', value: 'blue', color: '#dbeafe', darkColor: '#658b9f' },
    { label: '초록색', value: 'green', color: '#dcfce7', darkColor: '#7c9381' },
    { label: '분홍색', value: 'pink', color: '#fce7f3', darkColor: '#9a7b8d' },
    { label: '주황색', value: 'orange', color: '#fed7aa', darkColor: '#a4846a' },
    { label: '보라색', value: 'purple', color: '#e9d5ff', darkColor: '#8f7ea0' },
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
    { label: '작게', value: 'small', size: 24 },
    { label: '보통', value: 'medium', size: 48 },
    { label: '크게', value: 'large', size: 96 },
  ],

  fontFamily: [
    { label: '손글씨', value: 'handwriting', fontFamily: 'Dovemayo' },
    { label: '고딕', value: 'sans', fontFamily: 'Paperlogy' },
  ],
} as const;

export const defaultValues = {
  backgroundColor: 'purple',
  backgroundStyle: 'solid',
  roughness: 'rough',
  borderRadius: 'round',
  fontSize: 'medium',
  fontFamily: 'handwriting',
} as const;

export type BackgroundColor = (typeof values.backgroundColor)[number]['value'];
export type BackgroundStyle = (typeof values.backgroundStyle)[number]['value'];
export type Roughness = (typeof values.roughness)[number]['value'];
export type BorderRadius = (typeof values.borderRadius)[number]['value'];
export type FontSize = (typeof values.fontSize)[number]['value'];
export type FontFamily = (typeof values.fontFamily)[number]['value'];
