import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:sentry_flutter/sentry_flutter.dart';
import 'package:typie/error.dart';
import 'package:typie/providers/push_notification.dart';
import 'package:typie/routers/app.dart';
import 'package:typie/routers/observer.dart';
import 'package:typie/styles/colors.dart';

class App extends HookWidget {
  const App({super.key});

  @override
  Widget build(BuildContext context) {
    final router = useMemoized(AppRouter.new);

    const defaultTextStyle = TextStyle(
      fontFamily: 'Interop',
      fontSize: 16,
      color: AppColors.gray_950,
      height: 1.4,
      letterSpacing: 0,
    );

    return SentryWidget(
      child: MaterialApp.router(
        routerConfig: router.config(navigatorObservers: () => [RouterObserver(), SentryNavigatorObserver()]),
        debugShowCheckedModeBanner: false,
        theme: ThemeData(
          primaryColor: AppColors.gray_950,
          scaffoldBackgroundColor: AppColors.white,
          fontFamily: 'Interop',
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
          textSelectionTheme: TextSelectionThemeData(
            cursorColor: AppColors.gray_950,
            selectionColor: AppColors.gray_950.withValues(alpha: 0.15),
            selectionHandleColor: AppColors.gray_950,
          ),
          iconTheme: const IconThemeData(size: 24, color: AppColors.gray_950),
          progressIndicatorTheme: const ProgressIndicatorThemeData(strokeWidth: 1, color: AppColors.gray_950),
          cupertinoOverrideTheme: const CupertinoThemeData(primaryColor: AppColors.gray_950),
        ),
        builder: (context, child) {
          ErrorWidget.builder = (details) {
            return const AppErrorWidget();
          };

          return Overlay(
            initialEntries: [
              OverlayEntry(
                builder: (context) {
                  return Stack(
                    children: [
                      child!,
                      const Offstage(child: Stack(children: [PushNotificationProvider()])),
                    ],
                  );
                },
              ),
            ],
          );
        },
      ),
    );
  }
}
