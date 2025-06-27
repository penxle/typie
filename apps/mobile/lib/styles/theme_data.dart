import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/styles/semantic_colors.dart';

const _lightDefaultTextStyle = TextStyle(
  fontFamily: 'Interop',
  fontSize: 16,
  color: AppColors.gray_950,
  height: 1.4,
  letterSpacing: 0,
);

final lightTheme = ThemeData(
  brightness: Brightness.light,
  scaffoldBackgroundColor: AppColors.white,
  primaryColor: AppColors.gray_950,
  fontFamily: 'Interop',
  colorScheme: ColorScheme.fromSeed(
    seedColor: AppColors.brand_600,
  ),
  extensions: const [SemanticColors.light],
  appBarTheme: const AppBarTheme(
    backgroundColor: AppColors.white,
    foregroundColor: AppColors.gray_950,
    elevation: 0,
    titleTextStyle: TextStyle(
      fontFamily: 'Interop',
      color: AppColors.gray_950,
      fontSize: 18,
      fontWeight: FontWeight.w600,
    ),
  ),
  textTheme: const TextTheme(
    displaySmall: _lightDefaultTextStyle,
    displayMedium: _lightDefaultTextStyle,
    displayLarge: _lightDefaultTextStyle,
    headlineSmall: _lightDefaultTextStyle,
    headlineMedium: _lightDefaultTextStyle,
    headlineLarge: _lightDefaultTextStyle,
    titleSmall: _lightDefaultTextStyle,
    titleMedium: _lightDefaultTextStyle,
    titleLarge: _lightDefaultTextStyle,
    bodySmall: _lightDefaultTextStyle,
    bodyMedium: _lightDefaultTextStyle,
    bodyLarge: _lightDefaultTextStyle,
    labelSmall: _lightDefaultTextStyle,
    labelMedium: _lightDefaultTextStyle,
    labelLarge: _lightDefaultTextStyle,
  ),
  textSelectionTheme: TextSelectionThemeData(
    cursorColor: AppColors.gray_950,
    selectionColor: AppColors.gray_950.withValues(alpha: 0.15),
    selectionHandleColor: AppColors.gray_950,
  ),
  iconTheme: const IconThemeData(size: 24, color: AppColors.gray_950),
  progressIndicatorTheme: const ProgressIndicatorThemeData(strokeWidth: 1, color: AppColors.gray_950),
  cupertinoOverrideTheme: const CupertinoThemeData(primaryColor: AppColors.gray_950),
  dividerTheme: const DividerThemeData(
    color: AppColors.gray_200,
    thickness: 1,
  ),
);

final _darkDefaultTextStyle = TextStyle(
  fontFamily: 'Interop',
  fontSize: 16,
  color: AppColors.dark.gray_50,
  height: 1.4,
  letterSpacing: 0,
);

final darkTheme = ThemeData(
  brightness: Brightness.dark,
  scaffoldBackgroundColor: AppColors.dark.gray_900,
  primaryColor: AppColors.dark.gray_50,
  fontFamily: 'Interop',
  colorScheme: ColorScheme.fromSeed(
    seedColor: AppColors.dark.brand_600,
    brightness: Brightness.dark,
  ),
  extensions: [SemanticColors.dark],
  appBarTheme: AppBarTheme(
    backgroundColor: AppColors.dark.gray_800,
    foregroundColor: AppColors.dark.gray_50,
    elevation: 0,
    titleTextStyle: TextStyle(
      fontFamily: 'Interop',
      color: AppColors.dark.gray_50,
      fontSize: 18,
      fontWeight: FontWeight.w600,
    ),
  ),
  textTheme: TextTheme(
    displaySmall: _darkDefaultTextStyle,
    displayMedium: _darkDefaultTextStyle,
    displayLarge: _darkDefaultTextStyle,
    headlineSmall: _darkDefaultTextStyle,
    headlineMedium: _darkDefaultTextStyle,
    headlineLarge: _darkDefaultTextStyle,
    titleSmall: _darkDefaultTextStyle,
    titleMedium: _darkDefaultTextStyle,
    titleLarge: _darkDefaultTextStyle,
    bodySmall: _darkDefaultTextStyle,
    bodyMedium: _darkDefaultTextStyle,
    bodyLarge: _darkDefaultTextStyle,
    labelSmall: _darkDefaultTextStyle,
    labelMedium: _darkDefaultTextStyle,
    labelLarge: _darkDefaultTextStyle,
  ),
  textSelectionTheme: TextSelectionThemeData(
    cursorColor: AppColors.dark.gray_50,
    selectionColor: AppColors.dark.gray_50.withValues(alpha: 0.15),
    selectionHandleColor: AppColors.dark.gray_50,
  ),
  iconTheme: IconThemeData(size: 24, color: AppColors.dark.gray_50),
  progressIndicatorTheme: ProgressIndicatorThemeData(strokeWidth: 1, color: AppColors.dark.gray_50),
  cupertinoOverrideTheme: CupertinoThemeData(primaryColor: AppColors.dark.gray_50),
  dividerTheme: DividerThemeData(
    color: AppColors.dark.gray_700,
    thickness: 1,
  ),
);
