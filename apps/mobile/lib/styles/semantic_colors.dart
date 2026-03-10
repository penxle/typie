import 'package:flutter/material.dart';
import 'package:typie/styles/colors.dart';

@immutable
class SemanticColors extends ThemeExtension<SemanticColors> {
  const SemanticColors({
    required this.textDefault,
    required this.textSubtle,
    required this.textMuted,
    required this.textFaint,
    required this.textDisabled,
    required this.textDanger,
    required this.textSuccess,
    required this.textLink,
    required this.textBrand,
    required this.textBright,
    required this.textInverse,

    required this.surfaceDefault,
    required this.surfaceSubtle,
    required this.surfaceMuted,
    required this.surfaceDark,
    required this.surfaceInverse,

    required this.interactiveHover,
    required this.interactiveDisabled,

    required this.borderDefault,
    required this.borderSubtle,
    required this.borderStrong,
    required this.borderBrand,
    required this.borderDanger,
    required this.borderInverse,

    required this.accentBrand,
    required this.accentBrandHover,
    required this.accentBrandActive,
    required this.accentBrandSubtle,
    required this.accentInfo,
    required this.accentInfoSubtle,
    required this.accentDanger,
    required this.accentDangerHover,
    required this.accentDangerActive,
    required this.accentDangerSubtle,
    required this.accentSuccess,
    required this.accentSuccessSubtle,

    required this.shadowDefault,
    required this.overlayDefault,
  });

  final Color textDefault;
  final Color textSubtle;
  final Color textMuted;
  final Color textFaint;
  final Color textDisabled;
  final Color textDanger;
  final Color textSuccess;
  final Color textLink;
  final Color textBrand;
  final Color textBright;
  final Color textInverse;

  final Color surfaceDefault;
  final Color surfaceSubtle;
  final Color surfaceMuted;
  final Color surfaceDark;
  final Color surfaceInverse;

  final Color interactiveHover;
  final Color interactiveDisabled;

  final Color borderDefault;
  final Color borderSubtle;
  final Color borderStrong;
  final Color borderBrand;
  final Color borderDanger;
  final Color borderInverse;

  final Color accentBrand;
  final Color accentBrandHover;
  final Color accentBrandActive;
  final Color accentBrandSubtle;
  final Color accentInfo;
  final Color accentInfoSubtle;
  final Color accentDanger;
  final Color accentDangerHover;
  final Color accentDangerActive;
  final Color accentDangerSubtle;
  final Color accentSuccess;
  final Color accentSuccessSubtle;

  final Color shadowDefault;
  final Color overlayDefault;

  static const light = SemanticColors(
    textDefault: AppColors.gray_900,
    textSubtle: AppColors.gray_700,
    textMuted: AppColors.gray_600,
    textFaint: AppColors.gray_500,
    textDisabled: AppColors.gray_400,
    textDanger: AppColors.red_500,
    textSuccess: AppColors.green_700,
    textLink: AppColors.blue_600,
    textBrand: AppColors.brand_500,
    textBright: AppColors.white,
    textInverse: AppColors.white,

    surfaceDefault: AppColors.white,
    surfaceSubtle: AppColors.gray_50,
    surfaceMuted: AppColors.gray_100,
    surfaceDark: AppColors.gray_700,
    surfaceInverse: AppColors.gray_950,

    interactiveHover: AppColors.gray_200,
    interactiveDisabled: AppColors.gray_200,

    borderDefault: AppColors.gray_200,
    borderSubtle: AppColors.gray_100,
    borderStrong: AppColors.gray_300,
    borderBrand: AppColors.brand_600,
    borderDanger: AppColors.red_600,
    borderInverse: AppColors.gray_950,

    accentBrand: AppColors.brand_500,
    accentBrandHover: AppColors.brand_600,
    accentBrandActive: AppColors.brand_700,
    accentBrandSubtle: AppColors.brand_100,
    accentInfo: AppColors.blue_500,
    accentInfoSubtle: AppColors.blue_50,
    accentDanger: AppColors.red_600,
    accentDangerHover: AppColors.red_500,
    accentDangerActive: AppColors.red_700,
    accentDangerSubtle: AppColors.red_50,
    accentSuccess: AppColors.green_700,
    accentSuccessSubtle: AppColors.green_50,

    shadowDefault: AppColors.gray_950,
    overlayDefault: AppColors.black,
  );

  static final dark = SemanticColors(
    textDefault: AppColors.dark.gray_50,
    textSubtle: AppColors.dark.gray_100,
    textMuted: AppColors.dark.gray_200,
    textFaint: AppColors.dark.gray_300,
    textDisabled: AppColors.dark.gray_400,
    textDanger: AppColors.dark.red_300,
    textSuccess: AppColors.dark.green_300,
    textLink: AppColors.dark.blue_400,
    textBrand: AppColors.dark.brand_300,
    textBright: AppColors.dark.gray_50,
    textInverse: AppColors.dark.gray_900,

    surfaceDefault: AppColors.dark.gray_950,
    surfaceSubtle: AppColors.dark.gray_900,
    surfaceMuted: AppColors.dark.gray_800,
    surfaceDark: AppColors.dark.gray_700,
    surfaceInverse: AppColors.dark.gray_100,

    interactiveHover: AppColors.dark.gray_600,
    interactiveDisabled: AppColors.dark.gray_800,

    borderDefault: AppColors.dark.gray_700,
    borderSubtle: AppColors.dark.gray_800,
    borderStrong: AppColors.dark.gray_600,
    borderBrand: AppColors.dark.brand_400,
    borderDanger: AppColors.dark.red_400,
    borderInverse: AppColors.dark.gray_50,

    accentBrand: AppColors.dark.brand_400,
    accentBrandHover: AppColors.dark.brand_500,
    accentBrandActive: AppColors.dark.brand_600,
    accentBrandSubtle: AppColors.dark.brand_900,
    accentInfo: AppColors.dark.blue_200,
    accentInfoSubtle: AppColors.dark.blue_900,
    accentDanger: AppColors.dark.red_400,
    accentDangerHover: AppColors.dark.red_500,
    accentDangerActive: AppColors.dark.red_600,
    accentDangerSubtle: AppColors.dark.red_900,
    accentSuccess: AppColors.dark.green_300,
    accentSuccessSubtle: AppColors.dark.green_900,

    shadowDefault: AppColors.dark.gray_950,
    overlayDefault: AppColors.black,
  );

  @override
  SemanticColors copyWith({
    Color? textDefault,
    Color? textSubtle,
    Color? textMuted,
    Color? textFaint,
    Color? textDisabled,
    Color? textDanger,
    Color? textSuccess,
    Color? textLink,
    Color? textBrand,
    Color? textBright,
    Color? textInverse,

    Color? surfaceDefault,
    Color? surfaceSubtle,
    Color? surfaceMuted,
    Color? surfaceDark,
    Color? surfaceInverse,

    Color? interactiveHover,
    Color? interactiveDisabled,

    Color? borderDefault,
    Color? borderSubtle,
    Color? borderStrong,
    Color? borderBrand,
    Color? borderDanger,
    Color? borderInverse,

    Color? accentBrand,
    Color? accentBrandHover,
    Color? accentBrandActive,
    Color? accentBrandSubtle,
    Color? accentInfo,
    Color? accentInfoSubtle,
    Color? accentDanger,
    Color? accentDangerHover,
    Color? accentDangerActive,
    Color? accentDangerSubtle,
    Color? accentSuccess,
    Color? accentSuccessSubtle,

    Color? shadowDefault,
    Color? overlayDefault,
  }) {
    return SemanticColors(
      textDefault: textDefault ?? this.textDefault,
      textSubtle: textSubtle ?? this.textSubtle,
      textMuted: textMuted ?? this.textMuted,
      textFaint: textFaint ?? this.textFaint,
      textDisabled: textDisabled ?? this.textDisabled,
      textDanger: textDanger ?? this.textDanger,
      textSuccess: textSuccess ?? this.textSuccess,
      textLink: textLink ?? this.textLink,
      textBrand: textBrand ?? this.textBrand,
      textBright: textBright ?? this.textBright,
      textInverse: textInverse ?? this.textInverse,

      surfaceDefault: surfaceDefault ?? this.surfaceDefault,
      surfaceSubtle: surfaceSubtle ?? this.surfaceSubtle,
      surfaceMuted: surfaceMuted ?? this.surfaceMuted,
      surfaceDark: surfaceDark ?? this.surfaceDark,
      surfaceInverse: surfaceInverse ?? this.surfaceInverse,

      interactiveHover: interactiveHover ?? this.interactiveHover,
      interactiveDisabled: interactiveDisabled ?? this.interactiveDisabled,

      borderDefault: borderDefault ?? this.borderDefault,
      borderSubtle: borderSubtle ?? this.borderSubtle,
      borderStrong: borderStrong ?? this.borderStrong,
      borderBrand: borderBrand ?? this.borderBrand,
      borderDanger: borderDanger ?? this.borderDanger,
      borderInverse: borderInverse ?? this.borderInverse,

      accentBrand: accentBrand ?? this.accentBrand,
      accentBrandHover: accentBrandHover ?? this.accentBrandHover,
      accentBrandActive: accentBrandActive ?? this.accentBrandActive,
      accentBrandSubtle: accentBrandSubtle ?? this.accentBrandSubtle,
      accentInfo: accentInfo ?? this.accentInfo,
      accentInfoSubtle: accentInfoSubtle ?? this.accentInfoSubtle,
      accentDanger: accentDanger ?? this.accentDanger,
      accentDangerHover: accentDangerHover ?? this.accentDangerHover,
      accentDangerActive: accentDangerActive ?? this.accentDangerActive,
      accentDangerSubtle: accentDangerSubtle ?? this.accentDangerSubtle,
      accentSuccess: accentSuccess ?? this.accentSuccess,
      accentSuccessSubtle: accentSuccessSubtle ?? this.accentSuccessSubtle,

      shadowDefault: shadowDefault ?? this.shadowDefault,
      overlayDefault: overlayDefault ?? this.overlayDefault,
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
      textMuted: Color.lerp(textMuted, other.textMuted, t)!,
      textFaint: Color.lerp(textFaint, other.textFaint, t)!,
      textDisabled: Color.lerp(textDisabled, other.textDisabled, t)!,
      textDanger: Color.lerp(textDanger, other.textDanger, t)!,
      textSuccess: Color.lerp(textSuccess, other.textSuccess, t)!,
      textLink: Color.lerp(textLink, other.textLink, t)!,
      textBrand: Color.lerp(textBrand, other.textBrand, t)!,
      textBright: Color.lerp(textBright, other.textBright, t)!,
      textInverse: Color.lerp(textInverse, other.textInverse, t)!,

      surfaceDefault: Color.lerp(surfaceDefault, other.surfaceDefault, t)!,
      surfaceSubtle: Color.lerp(surfaceSubtle, other.surfaceSubtle, t)!,
      surfaceMuted: Color.lerp(surfaceMuted, other.surfaceMuted, t)!,
      surfaceDark: Color.lerp(surfaceDark, other.surfaceDark, t)!,
      surfaceInverse: Color.lerp(surfaceInverse, other.surfaceInverse, t)!,

      interactiveHover: Color.lerp(interactiveHover, other.interactiveHover, t)!,
      interactiveDisabled: Color.lerp(interactiveDisabled, other.interactiveDisabled, t)!,

      borderDefault: Color.lerp(borderDefault, other.borderDefault, t)!,
      borderSubtle: Color.lerp(borderSubtle, other.borderSubtle, t)!,
      borderStrong: Color.lerp(borderStrong, other.borderStrong, t)!,
      borderBrand: Color.lerp(borderBrand, other.borderBrand, t)!,
      borderDanger: Color.lerp(borderDanger, other.borderDanger, t)!,
      borderInverse: Color.lerp(borderInverse, other.borderInverse, t)!,

      accentBrand: Color.lerp(accentBrand, other.accentBrand, t)!,
      accentBrandHover: Color.lerp(accentBrandHover, other.accentBrandHover, t)!,
      accentBrandActive: Color.lerp(accentBrandActive, other.accentBrandActive, t)!,
      accentBrandSubtle: Color.lerp(accentBrandSubtle, other.accentBrandSubtle, t)!,
      accentInfo: Color.lerp(accentInfo, other.accentInfo, t)!,
      accentInfoSubtle: Color.lerp(accentInfoSubtle, other.accentInfoSubtle, t)!,
      accentDanger: Color.lerp(accentDanger, other.accentDanger, t)!,
      accentDangerHover: Color.lerp(accentDangerHover, other.accentDangerHover, t)!,
      accentDangerActive: Color.lerp(accentDangerActive, other.accentDangerActive, t)!,
      accentDangerSubtle: Color.lerp(accentDangerSubtle, other.accentDangerSubtle, t)!,
      accentSuccess: Color.lerp(accentSuccess, other.accentSuccess, t)!,
      accentSuccessSubtle: Color.lerp(accentSuccessSubtle, other.accentSuccessSubtle, t)!,

      shadowDefault: Color.lerp(shadowDefault, other.shadowDefault, t)!,
      overlayDefault: Color.lerp(overlayDefault, other.overlayDefault, t)!,
    );
  }
}
