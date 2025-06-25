import { defineSemanticTokens, defineTokens } from '@pandacss/dev';
import { aspectRatios } from './aspect-ratios';
import { blurs } from './blurs';
import { borderWidths } from './border-widths';
import { colors, semanticColors } from './colors';
import { fontSizes } from './font-sizes';
import { fontWeights } from './font-weights';
import { fonts } from './fonts';
import { gradients } from './gradients';
import { lineHeights } from './line-heights';
import { opacity } from './opacity';
import { radii } from './radii';
import { shadows } from './shadows';
import { sizes } from './sizes';
import { spacing } from './spacing';
import { zIndex } from './z-index';

export const tokens = defineTokens({
  aspectRatios,
  blurs,
  borderWidths,
  colors,
  fonts,
  fontSizes,
  fontWeights,
  gradients,
  lineHeights,
  opacity,
  radii,
  shadows,
  sizes,
  spacing,
  zIndex,
});

export const semanticTokens = defineSemanticTokens({
  colors: semanticColors,
});
