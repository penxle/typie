import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/providers/in_app_purchase.dart';
import 'package:typie/providers/push_notification.dart';
import 'package:typie/routers/app.dart';
import 'package:typie/styles/colors.dart';

class App extends HookWidget {
  const App({super.key});

  @override
  Widget build(BuildContext context) {
    final router = useMemoized(AppRouter.new);

    const defaultTextStyle = TextStyle(color: AppColors.gray_950, height: 1.4, letterSpacing: -0.015);

    return MaterialApp.router(
      routerConfig: router.config(),
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        fontFamily: 'SUIT',
        scaffoldBackgroundColor: AppColors.white,
        textTheme: const TextTheme(
          displaySmall: defaultTextStyle,
          displayMedium: defaultTextStyle,
          displayLarge: defaultTextStyle,
          headlineSmall: defaultTextStyle,
          headlineMedium: defaultTextStyle,
          headlineLarge: defaultTextStyle,
          titleSmall: defaultTextStyle,
          titleMedium: defaultTextStyle,
          titleLarge: defaultTextStyle,
          bodySmall: defaultTextStyle,
          bodyMedium: defaultTextStyle,
          bodyLarge: defaultTextStyle,
          labelSmall: defaultTextStyle,
          labelMedium: defaultTextStyle,
          labelLarge: defaultTextStyle,
        ),
        iconTheme: const IconThemeData(color: AppColors.gray_950, size: 24),
      ),
      builder: (context, child) {
        return Stack(
          children: [
            child!,
            const Offstage(child: Stack(children: [InAppPurchaseProvider(), PushNotificationProvider()])),
          ],
        );
      },
    );
  }
}
