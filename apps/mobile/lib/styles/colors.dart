import 'package:flutter/material.dart';

class AppColors {
  const AppColors._();

  static const white = Color(0xFFFFFFFF);
  static const black = Color(0xFF000000);
  static const transparent = Color(0x00000000);

  static const gray_50 = Color(0xFFF9FAFD);
  static const gray_100 = Color(0xFFF3F4F9);
  static const gray_200 = Color(0xFFE3E4EB);
  static const gray_300 = Color(0xFFD3D4DD);
  static const gray_400 = Color(0xFF9E9FA9);
  static const gray_500 = Color(0xFF70717B);
  static const gray_600 = Color(0xFF51525B);
  static const gray_700 = Color(0xFF3E3F47);
  static const gray_800 = Color(0xFF26272C);
  static const gray_900 = Color(0xFF17181C);
  static const gray_950 = Color(0xFF09090C);

  static const light = _LightColors();
  static const dark = _DarkColors();

  static const brand_50 = Color(0xFFF5F4FF);
  static const brand_100 = Color(0xFFE8E7FE);
  static const brand_200 = Color(0xFFD3D3FD);
  static const brand_300 = Color(0xFFB5B7F8);
  static const brand_400 = Color(0xFF9295E6);
  static const brand_500 = Color(0xFF6C6FC8);
  static const brand_600 = Color(0xFF5055AB);
  static const brand_700 = Color(0xFF3A408C);
  static const brand_800 = Color(0xFF2B3370);
  static const brand_900 = Color(0xFF1E2753);
  static const brand_950 = Color(0xFF101835);

  static const red_50 = Color(0xFFFEF2F4);
  static const red_100 = Color(0xFFFEE2E5);
  static const red_200 = Color(0xFFFFC9D0);
  static const red_300 = Color(0xFFFEA2AF);
  static const red_400 = Color(0xFFFA6781);
  static const red_500 = Color(0xFFF23864);
  static const red_600 = Color(0xFFE0024E);
  static const red_700 = Color(0xFFBB0440);
  static const red_800 = Color(0xFF9B0D34);
  static const red_900 = Color(0xFF80182E);
  static const red_950 = Color(0xFF450814);

  static const amber_50 = Color(0xFFFFFAEA);
  static const amber_100 = Color(0xFFFFF0C9);
  static const amber_200 = Color(0xFFFFDF94);
  static const amber_300 = Color(0xFFFFC850);
  static const amber_400 = Color(0xFFFFB31D);
  static const amber_500 = Color(0xFFF79D00);
  static const amber_600 = Color(0xFFD77A00);
  static const amber_700 = Color(0xFFAE5900);
  static const amber_800 = Color(0xFF8D4500);
  static const amber_900 = Color(0xFF74390A);
  static const amber_950 = Color(0xFF431C03);

  static const green_50 = Color(0xFFEFFFF6);
  static const green_100 = Color(0xFFDAFFEA);
  static const green_200 = Color(0xFFB9F5D4);
  static const green_300 = Color(0xFF79E8B1);
  static const green_400 = Color(0xFF00D185);
  static const green_500 = Color(0xFF00A96D);
  static const green_600 = Color(0xFF008857);
  static const green_700 = Color(0xFF00714A);
  static const green_800 = Color(0xFF005D3E);
  static const green_900 = Color(0xFF004731);
  static const green_950 = Color(0xFF003424);

  static const blue_50 = Color(0xFFEDF4FF);
  static const blue_100 = Color(0xFFDEEAFF);
  static const blue_200 = Color(0xFFC4D7FF);
  static const blue_300 = Color(0xFFA0BBFF);
  static const blue_400 = Color(0xFF7C99FF);
  static const blue_500 = Color(0xFF667AFF);
  static const blue_600 = Color(0xFF5661F3);
  static const blue_700 = Color(0xFF484CD5);
  static const blue_800 = Color(0xFF3C3CAD);
  static const blue_900 = Color(0xFF313389);
  static const blue_950 = Color(0xFF1A1B50);
}

class _LightColors {
  const _LightColors();

  final gray_50 = const Color(0xFFF9FAFD);
  final gray_100 = const Color(0xFFF3F4F9);
  final gray_200 = const Color(0xFFE3E4EB);
  final gray_300 = const Color(0xFFD3D4DD);
  final gray_400 = const Color(0xFF9E9FA9);
  final gray_500 = const Color(0xFF70717B);
  final gray_600 = const Color(0xFF51525B);
  final gray_700 = const Color(0xFF3E3F47);
  final gray_800 = const Color(0xFF26272C);
  final gray_900 = const Color(0xFF17181C);
  final gray_950 = const Color(0xFF09090C);

  final brand_50 = const Color(0xFFF5F4FF);
  final brand_100 = const Color(0xFFE8E7FE);
  final brand_200 = const Color(0xFFD3D3FD);
  final brand_300 = const Color(0xFFB5B7F8);
  final brand_400 = const Color(0xFF9295E6);
  final brand_500 = const Color(0xFF6C6FC8);
  final brand_600 = const Color(0xFF5055AB);
  final brand_700 = const Color(0xFF3A408C);
  final brand_800 = const Color(0xFF2B3370);
  final brand_900 = const Color(0xFF1E2753);
  final brand_950 = const Color(0xFF101835);

  final red_50 = const Color(0xFFFEF2F4);
  final red_100 = const Color(0xFFFEE2E5);
  final red_200 = const Color(0xFFFFC9D0);
  final red_300 = const Color(0xFFFEA2AF);
  final red_400 = const Color(0xFFFA6781);
  final red_500 = const Color(0xFFF23864);
  final red_600 = const Color(0xFFE0024E);
  final red_700 = const Color(0xFFBB0440);
  final red_800 = const Color(0xFF9B0D34);
  final red_900 = const Color(0xFF80182E);
  final red_950 = const Color(0xFF450814);

  final amber_50 = const Color(0xFFFFFAEA);
  final amber_100 = const Color(0xFFFFF0C9);
  final amber_200 = const Color(0xFFFFDF94);
  final amber_300 = const Color(0xFFFFC850);
  final amber_400 = const Color(0xFFFFB31D);
  final amber_500 = const Color(0xFFF79D00);
  final amber_600 = const Color(0xFFD77A00);
  final amber_700 = const Color(0xFFAE5900);
  final amber_800 = const Color(0xFF8D4500);
  final amber_900 = const Color(0xFF74390A);
  final amber_950 = const Color(0xFF431C03);

  final green_50 = const Color(0xFFEFFFF6);
  final green_100 = const Color(0xFFDAFFEA);
  final green_200 = const Color(0xFFB9F5D4);
  final green_300 = const Color(0xFF79E8B1);
  final green_400 = const Color(0xFF00D185);
  final green_500 = const Color(0xFF00A96D);
  final green_600 = const Color(0xFF008857);
  final green_700 = const Color(0xFF00714A);
  final green_800 = const Color(0xFF005D3E);
  final green_900 = const Color(0xFF004731);
  final green_950 = const Color(0xFF003424);

  final blue_50 = const Color(0xFFEDF4FF);
  final blue_100 = const Color(0xFFDEEAFF);
  final blue_200 = const Color(0xFFC4D7FF);
  final blue_300 = const Color(0xFFA0BBFF);
  final blue_400 = const Color(0xFF7C99FF);
  final blue_500 = const Color(0xFF667AFF);
  final blue_600 = const Color(0xFF5661F3);
  final blue_700 = const Color(0xFF484CD5);
  final blue_800 = const Color(0xFF3C3CAD);
  final blue_900 = const Color(0xFF313389);
  final blue_950 = const Color(0xFF1A1B50);
}

class _DarkColors {
  const _DarkColors();

  final gray_50 = const Color(0xFFF1F1F7);
  final gray_100 = const Color(0xFFDDDDE3);
  final gray_200 = const Color(0xFFC3C4C9);
  final gray_300 = const Color(0xFFA3A4A9);
  final gray_400 = const Color(0xFF7F8084);
  final gray_500 = const Color(0xFF5D5D62);
  final gray_600 = const Color(0xFF414246);
  final gray_700 = const Color(0xFF2D2D31);
  final gray_800 = const Color(0xFF1E1F23);
  final gray_900 = const Color(0xFF131317);
  final gray_950 = const Color(0xFF0A0B0E);

  final brand_50 = const Color(0xFFBDC0EE);
  final brand_100 = const Color(0xFFA3A9E0);
  final brand_200 = const Color(0xFF878FCF);
  final brand_300 = const Color(0xFF6974BB);
  final brand_400 = const Color(0xFF505CA4);
  final brand_500 = const Color(0xFF3C488E);
  final brand_600 = const Color(0xFF2E3A74);
  final brand_700 = const Color(0xFF202C5B);
  final brand_800 = const Color(0xFF172243);
  final brand_900 = const Color(0xFF0D172F);
  final brand_950 = const Color(0xFF080F1F);

  final red_50 = const Color(0xFFF3AFB6);
  final red_100 = const Color(0xFFE6939D);
  final red_200 = const Color(0xFFD87180);
  final red_300 = const Color(0xFFC64961);
  final red_400 = const Color(0xFFAE2749);
  final red_500 = const Color(0xFF990035);
  final red_600 = const Color(0xFF810027);
  final red_700 = const Color(0xFF67001B);
  final red_800 = const Color(0xFF4E0014);
  final red_900 = const Color(0xFF34020E);
  final red_950 = const Color(0xFF230208);

  final amber_50 = const Color(0xFFE9BA8D);
  final amber_100 = const Color(0xFFDAA168);
  final amber_200 = const Color(0xFFCC8231);
  final amber_300 = const Color(0xFFB66100);
  final amber_400 = const Color(0xFFA04600);
  final amber_500 = const Color(0xFF8D2B00);
  final amber_600 = const Color(0xFF741F00);
  final amber_700 = const Color(0xFF5C1300);
  final amber_800 = const Color(0xFF450F00);
  final amber_900 = const Color(0xFF2F0B00);
  final amber_950 = const Color(0xFF1F0700);

  final green_50 = const Color(0xFF86D9B0);
  final green_100 = const Color(0xFF45C992);
  final green_200 = const Color(0xFF00B276);
  final green_300 = const Color(0xFF00965C);
  final green_400 = const Color(0xFF007C47);
  final green_500 = const Color(0xFF006439);
  final green_600 = const Color(0xFF00512F);
  final green_700 = const Color(0xFF003E23);
  final green_800 = const Color(0xFF002E1C);
  final green_900 = const Color(0xFF001F13);
  final green_950 = const Color(0xFF00150C);

  final blue_50 = const Color(0xFFB2C2F9);
  final blue_100 = const Color(0xFF98AAEE);
  final blue_200 = const Color(0xFF7A8FE5);
  final blue_300 = const Color(0xFF5B70D8);
  final blue_400 = const Color(0xFF4455C2);
  final blue_500 = const Color(0xFF323DB0);
  final blue_600 = const Color(0xFF262D97);
  final blue_700 = const Color(0xFF1B1F7A);
  final blue_800 = const Color(0xFF14185C);
  final blue_900 = const Color(0xFF0D123C);
  final blue_950 = const Color(0xFF070B29);
}
