import { defineGlobalFontface, defineGlobalStyles } from '@pandacss/dev';

export const globalCss = defineGlobalStyles({
  '*': {
    margin: '0',
    padding: '0',

    font: 'inherit',
    lineHeight: 'inherit',
    letterSpacing: 'inherit',

    color: 'inherit',
    backgroundColor: 'transparent',

    textRendering: 'inherit',
  },

  '*, *::before, *::after': {
    boxSizing: 'border-box',
    border: '0 solid {colors.gray.200}',
    outline: '0 solid {colors.gray.200}',
  },

  html: {
    fontFamily: 'ui',
    fontFeatureSettings: '"ss05" 1, "cv12" 1, "ss18" 1',
    textSizeAdjust: '100%',

    color: 'gray.950',
    caretColor: 'gray.950',
    backgroundColor: 'white',

    lineHeight: '1.4',
    letterSpacing: '-0.015em',

    fontOpticalSizing: 'auto',
    WebkitFontSmoothing: 'antialiased',
    MozOsxFontSmoothing: 'grayscale',

    textRendering: 'optimizeLegibility',

    scrollbarWidth: 'thin',
    scrollbarColor: '{colors.gray.200} {colors.transparent}',

    WebkitTapHighlightColor: 'transparent',
  },

  body: {
    width: '[100dvw]',
    height: '[100dvh]',
    overflow: 'hidden',
  },

  a: {
    textDecoration: 'inherit',
  },

  button: {
    cursor: 'pointer',
    touchAction: 'manipulation',
  },

  hr: {
    height: '0',
  },

  'img, video': {
    display: 'block',
    maxWidth: 'full',
    height: 'auto',
  },

  input: {
    _disabled: {
      opacity: '100',
    },
  },

  'ol, ul': {
    listStyle: 'none',
  },

  ':disabled': {
    cursor: 'default',
  },

  ':focus-visible': {
    outline: 'none',
  },

  '::placeholder': {
    color: 'gray.300',
  },

  '::-webkit-search-cancel-button': {
    WebkitAppearance: 'none',
  },

  '[hidden]': {
    display: 'none!',
  },

  '::-webkit-details-marker': {
    display: 'none',
  },

  '::-webkit-scrollbar': {
    width: '10px',
    height: '10px',
  },

  '::-webkit-scrollbar-track': {
    backgroundColor: '{colors.transparent}',
  },

  '::-webkit-scrollbar-thumb': {
    borderWidth: '2px',
    borderStyle: 'solid',
    borderColor: '{colors.transparent}',
    borderRadius: 'full',
    backgroundColor: '{colors.gray.200}',
    backgroundClip: 'content-box',
  },

  '::-webkit-scrollbar-thumb:hover': {
    backgroundColor: '{colors.gray.300}',
  },
});

export const globalFontface = defineGlobalFontface({
  SUIT: {
    src: 'url("https://cdn.typie.net/fonts/SUIT-Variable.woff2") format("woff2-variations")',
    fontStyle: 'normal',
    fontWeight: '100 900',
    fontDisplay: 'swap',
  },

  IBMPlexSansKR: {
    src: 'url("https://cdn.typie.net/fonts/IBMPlexSansKR-Bold.woff2") format("woff2")',
    fontStyle: 'normal',
    fontWeight: '700',
    fontDisplay: 'swap',
  },

  LINESeedKR: [
    {
      src: 'url("https://cdn.typie.net/fonts/LINESeedKR-Regular.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '400',
      fontDisplay: 'swap',
    },
    {
      src: 'url("https://cdn.typie.net/fonts/LINESeedKR-Bold.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '700',
      fontDisplay: 'swap',
    },
  ],

  FiraCode: {
    src: 'url("https://cdn.typie.net/fonts/FiraCode-Variable.woff2") format("woff2-variations")',
    fontStyle: 'normal',
    fontWeight: '100 900',
    fontDisplay: 'swap',
  },

  KoPubWorldBatang: [
    {
      src: 'url("https://cdn.typie.net/fonts/KoPubWorldBatang-Medium.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '500',
      fontDisplay: 'block',
    },
    {
      src: 'url("https://cdn.typie.net/fonts/KoPubWorldBatang-Bold.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '700',
      fontDisplay: 'block',
    },
  ],

  KoPubWorldDotum: [
    {
      src: 'url("https://cdn.typie.net/fonts/KoPubWorldDotum-Medium.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '500',
      fontDisplay: 'block',
    },
    {
      src: 'url("https://cdn.typie.net/fonts/KoPubWorldDotum-Bold.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '700',
      fontDisplay: 'block',
    },
  ],

  NanumBarunGothic: [
    {
      src: 'url("https://cdn.typie.net/fonts/NanumBarunGothic-Regular.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '400',
      fontDisplay: 'block',
    },
    {
      src: 'url("https://cdn.typie.net/fonts/NanumBarunGothic-Bold.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '700',
      fontDisplay: 'block',
    },
  ],

  NanumMyeongjo: [
    {
      src: 'url("https://cdn.typie.net/fonts/NanumMyeongjo-Regular.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '400',
      fontDisplay: 'block',
    },
    {
      src: 'url("https://cdn.typie.net/fonts/NanumMyeongjo-Bold.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '700',
      fontDisplay: 'block',
    },
  ],

  Pretendard: {
    src: `url("https://cdn.typie.net/fonts/Pretendard-Variable.woff2") format("woff2-variations")`,
    fontStyle: 'normal',
    fontWeight: '100 900',
    fontDisplay: 'block',
  },

  RIDIBatang: {
    src: 'url("https://cdn.typie.net/fonts/RIDIBatang-Regular.woff2") format("woff2")',
    fontStyle: 'normal',
    fontWeight: '400',
    fontDisplay: 'block',
  },
});

export const globalVars = {};
