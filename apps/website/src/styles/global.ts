import { defineGlobalFontface, defineGlobalStyles } from '@pandacss/dev';
import pretendard from './pretendard.json' with { type: 'json' };

export const globalCss = defineGlobalStyles({
  '*': {
    margin: '0',
    padding: '0',
    font: 'inherit',
    color: 'inherit',
    backgroundColor: 'transparent',
  },

  '*, *::before, *::after': {
    boxSizing: 'border-box',
    border: '0 solid {colors.gray.200}',
    outline: '0 solid {colors.gray.200}',
  },

  html: {
    fontFamily: 'ui',
    fontFeatureSettings: '"ss18" 1',
    textSizeAdjust: '100%',

    WebkitFontSmoothing: 'antialiased',
    MozOsxFontSmoothing: 'grayscale',

    color: 'gray.950',
    backgroundColor: 'white',
    lineHeight: '1.4',
  },

  a: {
    textDecoration: 'inherit',
  },

  button: {
    cursor: 'pointer',
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
    color: 'gray.400',
  },

  '::-webkit-search-cancel-button': {
    WebkitAppearance: 'none',
  },

  '[hidden]': {
    display: 'none!',
  },
});

export const globalFontface = defineGlobalFontface({
  SUIT: {
    src: 'url("https://typie.net/fonts/SUIT.woff2") format("woff2-variations")',
    fontStyle: 'normal',
    fontWeight: '100 900',
    fontDisplay: 'swap',
  },
  IBMPlexSansKR: {
    src: 'url("https://typie.net/fonts/IBMPlexSansKR-Bold.woff2") format("woff2")',
    fontStyle: 'normal',
    fontWeight: '700',
    fontDisplay: 'swap',
  },
  LINESeedKR: [
    {
      src: 'url("https://typie.net/fonts/LINESeedKR-Rg.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '400',
      fontDisplay: 'swap',
    },
    {
      src: 'url("https://typie.net/fonts/LINESeedKR-Bd.woff2") format("woff2")',
      fontStyle: 'normal',
      fontWeight: '700',
      fontDisplay: 'swap',
    },
  ],
  Pretendard: pretendard.map((range, index) => ({
    src: `url("https://typie.net/fonts/PretendardVariable.subset.${index}.woff2") format("woff2-variations")`,
    fontStyle: 'normal',
    fontWeight: '100 900',
    fontDisplay: 'swap',
    unicodeRange: range,
  })),
  FiraCode: {
    src: 'url("https://typie.net/fonts/FiraCode.woff2") format("woff2-variations")',
    fontStyle: 'normal',
    fontWeight: '100 900',
    fontDisplay: 'swap',
  },
});

export const globalVars = {};
