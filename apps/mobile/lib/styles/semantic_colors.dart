import 'package:flutter/material.dart';
import 'package:typie/styles/colors.dart';

@immutable
class SemanticColors extends ThemeExtension<SemanticColors> {
  const SemanticColors({
    // Text colors (includes icons)
    required this.textDefault,
    required this.textSubtle,
    required this.textMuted,
    required this.textFaint,
    required this.textDisabled,
    required this.textInverse,
    required this.textBrand,
    required this.textDanger,
    required this.textSuccess,
    required this.textInfo,
    required this.textPlaceholder,
    required this.textOnBrand,
    required this.textOnDanger,
    required this.textOnToast,
    required this.textActive,

    // Surface colors
    required this.surfaceDefault,
    required this.surfaceSubtle,
    required this.surfaceMuted,
    required this.surfaceOverlay,
    required this.surfaceSelected,
    required this.surfaceToast,
    required this.surfaceModal,
    required this.surfaceDisabled,

    // Border colors
    required this.borderDefault,
    required this.borderStrong,
    required this.borderSubtle,
    required this.borderInput,
    required this.borderDivider,
    required this.borderModal,

    // Interactive colors
    required this.interactiveHover,
    required this.interactivePressed,
    required this.interactiveDisabled,
    required this.interactiveFocus,

    // Accent colors
    required this.accentBrandDefault,
    required this.accentBrandHover,
    required this.accentBrandPressed,
    required this.accentBrandSubtle,
    required this.accentDangerDefault,
    required this.accentDangerHover,
    required this.accentDangerPressed,
    required this.accentDangerSubtle,
    required this.accentSuccessDefault,
    required this.accentSuccessSubtle,
    required this.accentInfoDefault,

    // Control colors
    required this.controlBackground,
    required this.controlBorder,
    required this.controlPlaceholder,

    // Shadow colors
    required this.shadowDefault,
    required this.shadowOverlay,
  });

  // Text colors (includes icons)
  final Color textDefault;
  final Color textSubtle;
  final Color textMuted;
  final Color textFaint;
  final Color textDisabled;
  final Color textInverse;
  final Color textBrand;
  final Color textDanger;
  final Color textSuccess;
  final Color textInfo;
  final Color textPlaceholder;
  final Color textOnBrand;
  final Color textOnDanger;
  final Color textOnToast;
  final Color textActive;

  // Surface colors
  final Color surfaceDefault;
  final Color surfaceSubtle;
  final Color surfaceMuted;
  final Color surfaceOverlay;
  final Color surfaceSelected;
  final Color surfaceToast;
  final Color surfaceModal;
  final Color surfaceDisabled;

  // Border colors
  final Color borderDefault;
  final Color borderStrong;
  final Color borderSubtle;
  final Color borderInput;
  final Color borderDivider;
  final Color borderModal;

  // Interactive colors
  final Color interactiveHover;
  final Color interactivePressed;
  final Color interactiveDisabled;
  final Color interactiveFocus;

  // Accent colors
  final Color accentBrandDefault;
  final Color accentBrandHover;
  final Color accentBrandPressed;
  final Color accentBrandSubtle;
  final Color accentDangerDefault;
  final Color accentDangerHover;
  final Color accentDangerPressed;
  final Color accentDangerSubtle;
  final Color accentSuccessDefault;
  final Color accentSuccessSubtle;
  final Color accentInfoDefault;

  // Control colors
  final Color controlBackground;
  final Color controlBorder;
  final Color controlPlaceholder;

  // Shadow colors
  final Color shadowDefault;
  final Color shadowOverlay;

  static const light = SemanticColors(
    // Text colors (includes icons)
    textDefault: AppColors.gray_950,
    textSubtle: AppColors.gray_700,
    textMuted: AppColors.gray_600,
    textFaint: AppColors.gray_500,
    textDisabled: AppColors.gray_400,
    textInverse: AppColors.white,
    textBrand: AppColors.brand_600,
    textDanger: AppColors.red_500,
    textSuccess: AppColors.green_500,
    textInfo: AppColors.blue_600,
    textPlaceholder: AppColors.gray_500,
    textOnBrand: AppColors.white,
    textOnDanger: AppColors.white,
    textOnToast: AppColors.white,
    textActive: AppColors.gray_950,

    // Surface colors
    surfaceDefault: AppColors.white,
    surfaceSubtle: AppColors.gray_50,
    surfaceMuted: AppColors.gray_100,
    surfaceOverlay: AppColors.black,
    surfaceSelected: AppColors.gray_100,
    surfaceToast: AppColors.gray_950,
    surfaceModal: AppColors.white,
    surfaceDisabled: AppColors.gray_400,

    // Border colors
    borderDefault: AppColors.gray_200,
    borderStrong: AppColors.gray_950,
    borderSubtle: AppColors.gray_100,
    borderInput: AppColors.gray_200,
    borderDivider: AppColors.gray_300,
    borderModal: AppColors.gray_950,

    // Interactive colors
    interactiveHover: AppColors.gray_100,
    interactivePressed: AppColors.gray_200,
    interactiveDisabled: AppColors.gray_200,
    interactiveFocus: AppColors.brand_500,

    // Accent colors
    accentBrandDefault: AppColors.brand_500,
    accentBrandHover: AppColors.brand_600,
    accentBrandPressed: AppColors.brand_700,
    accentBrandSubtle: AppColors.brand_100,
    accentDangerDefault: AppColors.red_500,
    accentDangerHover: AppColors.red_600,
    accentDangerPressed: AppColors.red_700,
    accentDangerSubtle: AppColors.red_100,
    accentSuccessDefault: AppColors.green_500,
    accentSuccessSubtle: AppColors.green_100,
    accentInfoDefault: AppColors.blue_600,

    // Control colors
    controlBackground: AppColors.white,
    controlBorder: AppColors.gray_200,
    controlPlaceholder: AppColors.gray_500,

    // Shadow colors
    shadowDefault: AppColors.black,
    shadowOverlay: AppColors.black,
  );

  static final dark = SemanticColors(
    // Text colors (includes icons)
    textDefault: AppColors.dark.gray_50,
    textSubtle: AppColors.dark.gray_100,
    textMuted: AppColors.dark.gray_200,
    textFaint: AppColors.dark.gray_300,
    textDisabled: AppColors.dark.gray_400,
    textInverse: AppColors.dark.gray_900,
    textBrand: AppColors.dark.brand_400,
    textDanger: AppColors.dark.red_400,
    textSuccess: AppColors.dark.green_400,
    textInfo: AppColors.dark.blue_400,
    textPlaceholder: AppColors.dark.gray_300,
    textOnBrand: AppColors.white,
    textOnDanger: AppColors.white,
    textOnToast: AppColors.dark.gray_50,
    textActive: AppColors.dark.gray_50,

    // Surface colors
    surfaceDefault: AppColors.dark.gray_900,
    surfaceSubtle: AppColors.dark.gray_800,
    surfaceMuted: AppColors.dark.gray_700,
    surfaceOverlay: AppColors.dark.gray_700,
    surfaceSelected: AppColors.dark.gray_700,
    surfaceToast: AppColors.dark.gray_700,
    surfaceModal: AppColors.dark.gray_800,
    surfaceDisabled: AppColors.dark.gray_800,

    // Border colors
    borderDefault: AppColors.dark.gray_700,
    borderStrong: AppColors.dark.gray_600,
    borderSubtle: AppColors.dark.gray_800,
    borderInput: AppColors.dark.gray_700,
    borderDivider: AppColors.dark.gray_700,
    borderModal: AppColors.dark.gray_600,

    // Interactive colors
    interactiveHover: AppColors.dark.gray_600,
    interactivePressed: AppColors.dark.gray_500,
    interactiveDisabled: AppColors.dark.gray_800,
    interactiveFocus: AppColors.dark.brand_400,

    // Accent colors
    accentBrandDefault: AppColors.dark.brand_400,
    accentBrandHover: AppColors.dark.brand_500,
    accentBrandPressed: AppColors.dark.brand_600,
    accentBrandSubtle: AppColors.dark.brand_900,
    accentDangerDefault: AppColors.dark.red_400,
    accentDangerHover: AppColors.dark.red_500,
    accentDangerPressed: AppColors.dark.red_600,
    accentDangerSubtle: AppColors.dark.red_900,
    accentSuccessDefault: AppColors.dark.green_400,
    accentSuccessSubtle: AppColors.dark.green_900,
    accentInfoDefault: AppColors.dark.blue_400,

    // Control colors
    controlBackground: AppColors.dark.gray_800,
    controlBorder: AppColors.dark.gray_700,
    controlPlaceholder: AppColors.dark.gray_300,

    // Shadow colors
    shadowDefault: AppColors.black,
    shadowOverlay: AppColors.black,
  );

  @override
  SemanticColors copyWith({
    // Text colors (includes icons)
    Color? textDefault,
    Color? textSubtle,
    Color? textMuted,
    Color? textFaint,
    Color? textDisabled,
    Color? textInverse,
    Color? textBrand,
    Color? textDanger,
    Color? textSuccess,
    Color? textInfo,
    Color? textPlaceholder,
    Color? textOnBrand,
    Color? textOnDanger,
    Color? textOnToast,
    Color? textActive,

    // Surface colors
    Color? surfaceDefault,
    Color? surfaceSubtle,
    Color? surfaceMuted,
    Color? surfaceOverlay,
    Color? surfaceSelected,
    Color? surfaceToast,
    Color? surfaceModal,
    Color? surfaceDisabled,

    // Border colors
    Color? borderDefault,
    Color? borderStrong,
    Color? borderSubtle,
    Color? borderInput,
    Color? borderDivider,
    Color? borderModal,

    // Interactive colors
    Color? interactiveHover,
    Color? interactivePressed,
    Color? interactiveDisabled,
    Color? interactiveFocus,

    // Accent colors
    Color? accentBrandDefault,
    Color? accentBrandHover,
    Color? accentBrandPressed,
    Color? accentBrandSubtle,
    Color? accentDangerDefault,
    Color? accentDangerHover,
    Color? accentDangerPressed,
    Color? accentDangerSubtle,
    Color? accentSuccessDefault,
    Color? accentSuccessSubtle,
    Color? accentInfoDefault,

    // Control colors
    Color? controlBackground,
    Color? controlBorder,
    Color? controlPlaceholder,

    // Shadow colors
    Color? shadowDefault,
    Color? shadowOverlay,
  }) {
    return SemanticColors(
      // Text colors (includes icons)
      textDefault: textDefault ?? this.textDefault,
      textSubtle: textSubtle ?? this.textSubtle,
      textMuted: textMuted ?? this.textMuted,
      textFaint: textFaint ?? this.textFaint,
      textDisabled: textDisabled ?? this.textDisabled,
      textInverse: textInverse ?? this.textInverse,
      textBrand: textBrand ?? this.textBrand,
      textDanger: textDanger ?? this.textDanger,
      textSuccess: textSuccess ?? this.textSuccess,
      textInfo: textInfo ?? this.textInfo,
      textPlaceholder: textPlaceholder ?? this.textPlaceholder,
      textOnBrand: textOnBrand ?? this.textOnBrand,
      textOnDanger: textOnDanger ?? this.textOnDanger,
      textOnToast: textOnToast ?? this.textOnToast,
      textActive: textActive ?? this.textActive,

      // Surface colors
      surfaceDefault: surfaceDefault ?? this.surfaceDefault,
      surfaceSubtle: surfaceSubtle ?? this.surfaceSubtle,
      surfaceMuted: surfaceMuted ?? this.surfaceMuted,
      surfaceOverlay: surfaceOverlay ?? this.surfaceOverlay,
      surfaceSelected: surfaceSelected ?? this.surfaceSelected,
      surfaceToast: surfaceToast ?? this.surfaceToast,
      surfaceModal: surfaceModal ?? this.surfaceModal,
      surfaceDisabled: surfaceDisabled ?? this.surfaceDisabled,

      // Border colors
      borderDefault: borderDefault ?? this.borderDefault,
      borderStrong: borderStrong ?? this.borderStrong,
      borderSubtle: borderSubtle ?? this.borderSubtle,
      borderInput: borderInput ?? this.borderInput,
      borderDivider: borderDivider ?? this.borderDivider,
      borderModal: borderModal ?? this.borderModal,

      // Interactive colors
      interactiveHover: interactiveHover ?? this.interactiveHover,
      interactivePressed: interactivePressed ?? this.interactivePressed,
      interactiveDisabled: interactiveDisabled ?? this.interactiveDisabled,
      interactiveFocus: interactiveFocus ?? this.interactiveFocus,

      // Accent colors
      accentBrandDefault: accentBrandDefault ?? this.accentBrandDefault,
      accentBrandHover: accentBrandHover ?? this.accentBrandHover,
      accentBrandPressed: accentBrandPressed ?? this.accentBrandPressed,
      accentBrandSubtle: accentBrandSubtle ?? this.accentBrandSubtle,
      accentDangerDefault: accentDangerDefault ?? this.accentDangerDefault,
      accentDangerHover: accentDangerHover ?? this.accentDangerHover,
      accentDangerPressed: accentDangerPressed ?? this.accentDangerPressed,
      accentDangerSubtle: accentDangerSubtle ?? this.accentDangerSubtle,
      accentSuccessDefault: accentSuccessDefault ?? this.accentSuccessDefault,
      accentSuccessSubtle: accentSuccessSubtle ?? this.accentSuccessSubtle,
      accentInfoDefault: accentInfoDefault ?? this.accentInfoDefault,

      // Control colors
      controlBackground: controlBackground ?? this.controlBackground,
      controlBorder: controlBorder ?? this.controlBorder,
      controlPlaceholder: controlPlaceholder ?? this.controlPlaceholder,

      // Shadow colors
      shadowDefault: shadowDefault ?? this.shadowDefault,
      shadowOverlay: shadowOverlay ?? this.shadowOverlay,
    );
  }

  @override
  ThemeExtension<SemanticColors> lerp(ThemeExtension<SemanticColors>? other, double t) {
    if (other is! SemanticColors) {
      return this;
    }
    return SemanticColors(
      // Text colors (includes icons)
      textDefault: Color.lerp(textDefault, other.textDefault, t)!,
      textSubtle: Color.lerp(textSubtle, other.textSubtle, t)!,
      textMuted: Color.lerp(textMuted, other.textMuted, t)!,
      textFaint: Color.lerp(textFaint, other.textFaint, t)!,
      textDisabled: Color.lerp(textDisabled, other.textDisabled, t)!,
      textInverse: Color.lerp(textInverse, other.textInverse, t)!,
      textBrand: Color.lerp(textBrand, other.textBrand, t)!,
      textDanger: Color.lerp(textDanger, other.textDanger, t)!,
      textSuccess: Color.lerp(textSuccess, other.textSuccess, t)!,
      textInfo: Color.lerp(textInfo, other.textInfo, t)!,
      textPlaceholder: Color.lerp(textPlaceholder, other.textPlaceholder, t)!,
      textOnBrand: Color.lerp(textOnBrand, other.textOnBrand, t)!,
      textOnDanger: Color.lerp(textOnDanger, other.textOnDanger, t)!,
      textOnToast: Color.lerp(textOnToast, other.textOnToast, t)!,
      textActive: Color.lerp(textActive, other.textActive, t)!,

      // Surface colors
      surfaceDefault: Color.lerp(surfaceDefault, other.surfaceDefault, t)!,
      surfaceSubtle: Color.lerp(surfaceSubtle, other.surfaceSubtle, t)!,
      surfaceMuted: Color.lerp(surfaceMuted, other.surfaceMuted, t)!,
      surfaceOverlay: Color.lerp(surfaceOverlay, other.surfaceOverlay, t)!,
      surfaceSelected: Color.lerp(surfaceSelected, other.surfaceSelected, t)!,
      surfaceToast: Color.lerp(surfaceToast, other.surfaceToast, t)!,
      surfaceModal: Color.lerp(surfaceModal, other.surfaceModal, t)!,
      surfaceDisabled: Color.lerp(surfaceDisabled, other.surfaceDisabled, t)!,

      // Border colors
      borderDefault: Color.lerp(borderDefault, other.borderDefault, t)!,
      borderStrong: Color.lerp(borderStrong, other.borderStrong, t)!,
      borderSubtle: Color.lerp(borderSubtle, other.borderSubtle, t)!,
      borderInput: Color.lerp(borderInput, other.borderInput, t)!,
      borderDivider: Color.lerp(borderDivider, other.borderDivider, t)!,
      borderModal: Color.lerp(borderModal, other.borderModal, t)!,

      // Interactive colors
      interactiveHover: Color.lerp(interactiveHover, other.interactiveHover, t)!,
      interactivePressed: Color.lerp(interactivePressed, other.interactivePressed, t)!,
      interactiveDisabled: Color.lerp(interactiveDisabled, other.interactiveDisabled, t)!,
      interactiveFocus: Color.lerp(interactiveFocus, other.interactiveFocus, t)!,

      // Accent colors
      accentBrandDefault: Color.lerp(accentBrandDefault, other.accentBrandDefault, t)!,
      accentBrandHover: Color.lerp(accentBrandHover, other.accentBrandHover, t)!,
      accentBrandPressed: Color.lerp(accentBrandPressed, other.accentBrandPressed, t)!,
      accentBrandSubtle: Color.lerp(accentBrandSubtle, other.accentBrandSubtle, t)!,
      accentDangerDefault: Color.lerp(accentDangerDefault, other.accentDangerDefault, t)!,
      accentDangerHover: Color.lerp(accentDangerHover, other.accentDangerHover, t)!,
      accentDangerPressed: Color.lerp(accentDangerPressed, other.accentDangerPressed, t)!,
      accentDangerSubtle: Color.lerp(accentDangerSubtle, other.accentDangerSubtle, t)!,
      accentSuccessDefault: Color.lerp(accentSuccessDefault, other.accentSuccessDefault, t)!,
      accentSuccessSubtle: Color.lerp(accentSuccessSubtle, other.accentSuccessSubtle, t)!,
      accentInfoDefault: Color.lerp(accentInfoDefault, other.accentInfoDefault, t)!,

      // Control colors
      controlBackground: Color.lerp(controlBackground, other.controlBackground, t)!,
      controlBorder: Color.lerp(controlBorder, other.controlBorder, t)!,
      controlPlaceholder: Color.lerp(controlPlaceholder, other.controlPlaceholder, t)!,

      // Shadow colors
      shadowDefault: Color.lerp(shadowDefault, other.shadowDefault, t)!,
      shadowOverlay: Color.lerp(shadowOverlay, other.shadowOverlay, t)!,
    );
  }
}
