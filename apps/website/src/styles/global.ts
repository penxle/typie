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
    border: '0 solid {colors.border.default}',
    outline: '0 solid {colors.border.default}',
  },

  html: {
    fontFamily: 'ui',
    fontFeatureSettings: '"ss05" 1, "cv12" 1, "ss18" 1',
    textSizeAdjust: '100%',

    color: 'text.default',
    caretColor: 'text.default',
    backgroundColor: 'surface.default',

    lineHeight: '1.4',
    letterSpacing: '-0.015em',

    fontOpticalSizing: 'auto',
    WebkitFontSmoothing: 'antialiased',
    MozOsxFontSmoothing: 'grayscale',

    textRendering: 'optimizeLegibility',

    scrollbarWidth: 'thin',
    scrollbarColor: '{colors.control.scrollbar.default} {colors.transparent}',

    WebkitTapHighlightColor: 'transparent',
  },

  body: {
    width: '[100dvw]',
    height: '[100dvh]',
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
    color: 'text.disabled',
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
    backgroundColor: '{colors.control.scrollbar.default}',
    backgroundClip: 'content-box',
  },

  '::-webkit-scrollbar-thumb:hover': {
    backgroundColor: '{colors.control.scrollbar.hover}',
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

  Interop: {
    src: 'url("https://cdn.typie.net/fonts/Interop-Regular.woff2") format("woff2")',
    fontStyle: 'normal',
    fontWeight: '400',
    fontDisplay: 'block',
  },

  Paperlogy: [
    {
      src: 'url("https://cdn.typie.net/fonts/Paperlogy-1Thin.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '100',
      fontDisplay: 'block',
    },
    {
      src: 'url("https://cdn.typie.net/fonts/Paperlogy-2ExtraLight.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '200',
      fontDisplay: 'block',
    },
    {
      src: 'url("https://cdn.typie.net/fonts/Paperlogy-3Light.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '300',
      fontDisplay: 'block',
    },
    {
      src: 'url("https://cdn.typie.net/fonts/Paperlogy-4Regular.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '400',
      fontDisplay: 'block',
    },
    {
      src: 'url("https://cdn.typie.net/fonts/Paperlogy-5Medium.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '500',
      fontDisplay: 'block',
    },
    {
      src: 'url("https://cdn.typie.net/fonts/Paperlogy-6SemiBold.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '600',
      fontDisplay: 'block',
    },
    {
      src: 'url("https://cdn.typie.net/fonts/Paperlogy-7Bold.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '700',
      fontDisplay: 'block',
    },
    {
      src: 'url("https://cdn.typie.net/fonts/Paperlogy-8ExtraBold.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '800',
      fontDisplay: 'block',
    },
    {
      src: 'url("https://cdn.typie.net/fonts/Paperlogy-9Black.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '900',
      fontDisplay: 'block',
    },
  ],

  Dovemayo: {
    src: 'url("https://cdn.typie.net/fonts/Dovemayo.woff2") format("woff2")',
    fontStyle: 'normal',
    fontWeight: '400',
    fontDisplay: 'block',
  },
});

export const globalVars = {};
