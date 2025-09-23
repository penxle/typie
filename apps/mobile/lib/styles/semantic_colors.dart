import 'package:flutter/material.dart';
import 'package:typie/styles/colors.dart';

@immutable
class SemanticColors extends ThemeExtension<SemanticColors> {
  const SemanticColors({
    required this.textDefault,
    required this.textSubtle,
    required this.textFaint,
    required this.textDisabled,
    required this.textDanger,
    required this.textBright,
    required this.textInverse,

    required this.surfaceDefault,
    required this.surfaceSubtle,
    required this.surfaceMuted,
    required this.surfaceDark,
    required this.surfaceInverse,

    required this.borderDefault,
    required this.borderSubtle,
    required this.borderStrong,
    required this.borderInverse,

    required this.accentBrand,
    required this.accentDanger,
    required this.accentSuccess,

    required this.shadowDefault,
    required this.overlayDefault,

    required this.prosemirrorBlack,
    required this.prosemirrorDarkgray,
    required this.prosemirrorLightgray,
    required this.prosemirrorWhite,
  });

  final Color textDefault;
  final Color textSubtle;
  final Color textFaint;
  final Color textDisabled;
  final Color textDanger;
  final Color textBright;
  final Color textInverse;

  final Color surfaceDefault;
  final Color surfaceSubtle;
  final Color surfaceMuted;
  final Color surfaceDark;
  final Color surfaceInverse;

  final Color borderDefault;
  final Color borderSubtle;
  final Color borderStrong;
  final Color borderInverse;

  final Color accentBrand;
  final Color accentDanger;
  final Color accentSuccess;

  final Color shadowDefault;
  final Color overlayDefault;

  final Color prosemirrorBlack;
  final Color prosemirrorDarkgray;
  final Color prosemirrorLightgray;
  final Color prosemirrorWhite;

  static const light = SemanticColors(
    textDefault: AppColors.gray_950,
    textSubtle: AppColors.gray_700,
    textFaint: AppColors.gray_500,
    textDisabled: AppColors.gray_400,
    textDanger: AppColors.red_500,
    textBright: AppColors.white,
    textInverse: AppColors.white,

    surfaceDefault: AppColors.white,
    surfaceSubtle: AppColors.gray_50,
    surfaceMuted: AppColors.gray_100,
    surfaceDark: AppColors.gray_950,
    surfaceInverse: AppColors.gray_950,

    borderDefault: AppColors.gray_200,
    borderSubtle: AppColors.gray_100,
    borderStrong: AppColors.gray_950,
    borderInverse: AppColors.gray_950,

    accentBrand: AppColors.brand_500,
    accentDanger: AppColors.red_500,
    accentSuccess: AppColors.green_500,

    shadowDefault: AppColors.black,
    overlayDefault: AppColors.black,

    prosemirrorBlack: AppColors.gray_900,
    prosemirrorDarkgray: AppColors.gray_600,
    prosemirrorLightgray: AppColors.gray_300,
    prosemirrorWhite: AppColors.white,
  );

  static final dark = SemanticColors(
    textDefault: AppColors.dark.gray_50,
    textSubtle: AppColors.dark.gray_100,
    textFaint: AppColors.dark.gray_300,
    textDisabled: AppColors.dark.gray_400,
    textDanger: AppColors.dark.red_400,
    textBright: AppColors.dark.gray_50,
    textInverse: AppColors.dark.gray_900,

    surfaceDefault: AppColors.dark.gray_900,
    surfaceSubtle: AppColors.dark.gray_800,
    surfaceMuted: AppColors.dark.gray_700,
    surfaceDark: AppColors.dark.gray_500,
    surfaceInverse: AppColors.dark.gray_100,

    borderDefault: AppColors.dark.gray_700,
    borderSubtle: AppColors.dark.gray_800,
    borderStrong: AppColors.dark.gray_500,
    borderInverse: AppColors.dark.gray_50,

    accentBrand: AppColors.dark.brand_400,
    accentDanger: AppColors.dark.red_400,
    accentSuccess: AppColors.dark.green_400,

    shadowDefault: AppColors.black,
    overlayDefault: AppColors.black,

    prosemirrorBlack: AppColors.dark.gray_50,
    prosemirrorDarkgray: AppColors.dark.gray_300,
    prosemirrorLightgray: AppColors.dark.gray_600,
    prosemirrorWhite: AppColors.dark.gray_900,
  );

  @override
  SemanticColors copyWith({
    Color? textDefault,
    Color? textSubtle,
    Color? textFaint,
    Color? textDisabled,
    Color? textDanger,
    Color? textBright,
    Color? textInverse,

    Color? surfaceDefault,
    Color? surfaceSubtle,
    Color? surfaceMuted,
    Color? surfaceDark,
    Color? surfaceInverse,

    Color? borderDefault,
    Color? borderSubtle,
    Color? borderStrong,
    Color? borderInverse,

    Color? accentBrand,
    Color? accentDanger,
    Color? accentSuccess,

    Color? shadowDefault,
    Color? overlayDefault,

    Color? prosemirrorBlack,
    Color? prosemirrorDarkgray,
    Color? prosemirrorLightgray,
    Color? prosemirrorWhite,
  }) {
    return SemanticColors(
      textDefault: textDefault ?? this.textDefault,
      textSubtle: textSubtle ?? this.textSubtle,
      textFaint: textFaint ?? this.textFaint,
      textDisabled: textDisabled ?? this.textDisabled,
      textDanger: textDanger ?? this.textDanger,
      textBright: textBright ?? this.textBright,
      textInverse: textInverse ?? this.textInverse,

      surfaceDefault: surfaceDefault ?? this.surfaceDefault,
      surfaceSubtle: surfaceSubtle ?? this.surfaceSubtle,
      surfaceMuted: surfaceMuted ?? this.surfaceMuted,
      surfaceDark: surfaceDark ?? this.surfaceDark,
      surfaceInverse: surfaceInverse ?? this.surfaceInverse,

      borderDefault: borderDefault ?? this.borderDefault,
      borderSubtle: borderSubtle ?? this.borderSubtle,
      borderStrong: borderStrong ?? this.borderStrong,
      borderInverse: borderInverse ?? this.borderInverse,

      accentBrand: accentBrand ?? this.accentBrand,
      accentDanger: accentDanger ?? this.accentDanger,
      accentSuccess: accentSuccess ?? this.accentSuccess,

      shadowDefault: shadowDefault ?? this.shadowDefault,
      overlayDefault: overlayDefault ?? this.overlayDefault,

      prosemirrorBlack: prosemirrorBlack ?? this.prosemirrorBlack,
      prosemirrorDarkgray: prosemirrorDarkgray ?? this.prosemirrorDarkgray,
      prosemirrorLightgray: prosemirrorLightgray ?? this.prosemirrorLightgray,
      prosemirrorWhite: prosemirrorWhite ?? this.prosemirrorWhite,
    );
  }

  @override
  ThemeExtension<SemanticColors> lerp(ThemeExtension<SemanticColors>? other, double t) {
    if (other is! SemanticColors) {
      return this;
    }
    return SemanticColors(
      textDefault: Color.lerp(textDefault, other.textDefault, t)!,
      textSubtle: Color.lerp(textSubtle, other.textSubtle, t)!,
      textFaint: Color.lerp(textFaint, other.textFaint, t)!,
      textDisabled: Color.lerp(textDisabled, other.textDisabled, t)!,
      textDanger: Color.lerp(textDanger, other.textDanger, t)!,
      textBright: Color.lerp(textBright, other.textBright, t)!,
      textInverse: Color.lerp(textInverse, other.textInverse, t)!,

      surfaceDefault: Color.lerp(surfaceDefault, other.surfaceDefault, t)!,
      surfaceSubtle: Color.lerp(surfaceSubtle, other.surfaceSubtle, t)!,
      surfaceMuted: Color.lerp(surfaceMuted, other.surfaceMuted, t)!,
      surfaceDark: Color.lerp(surfaceDark, other.surfaceDark, t)!,
      surfaceInverse: Color.lerp(surfaceInverse, other.surfaceInverse, t)!,

      borderDefault: Color.lerp(borderDefault, other.borderDefault, t)!,
      borderSubtle: Color.lerp(borderSubtle, other.borderSubtle, t)!,
      borderStrong: Color.lerp(borderStrong, other.borderStrong, t)!,
      borderInverse: Color.lerp(borderInverse, other.borderInverse, t)!,

      accentBrand: Color.lerp(accentBrand, other.accentBrand, t)!,
      accentDanger: Color.lerp(accentDanger, other.accentDanger, t)!,
      accentSuccess: Color.lerp(accentSuccess, other.accentSuccess, t)!,

      shadowDefault: Color.lerp(shadowDefault, other.shadowDefault, t)!,
      overlayDefault: Color.lerp(overlayDefault, other.overlayDefault, t)!,

      prosemirrorBlack: Color.lerp(prosemirrorBlack, other.prosemirrorBlack, t)!,
      prosemirrorDarkgray: Color.lerp(prosemirrorDarkgray, other.prosemirrorDarkgray, t)!,
      prosemirrorLightgray: Color.lerp(prosemirrorLightgray, other.prosemirrorLightgray, t)!,
      prosemirrorWhite: Color.lerp(prosemirrorWhite, other.prosemirrorWhite, t)!,
    );
  }
}
