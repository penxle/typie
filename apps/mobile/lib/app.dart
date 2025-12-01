import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:sentry_flutter/sentry_flutter.dart';
import 'package:typie/error.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/providers/in_app_purchase.dart';
import 'package:typie/providers/push_notification.dart';
import 'package:typie/routers/app.dart';
import 'package:typie/routers/observer.dart';
import 'package:typie/services/theme.dart';
import 'package:typie/styles/theme_data.dart';

class App extends HookWidget {
  const App({super.key});

  @override
  Widget build(BuildContext context) {
    final router = useMemoized(AppRouter.new);
    final theme = useService<AppTheme>();

    useListenable(theme);

    return SentryWidget(
      child: MaterialApp.router(
        routerConfig: router.config(navigatorObservers: () => [RouterObserver(), SentryNavigatorObserver()]),
        debugShowCheckedModeBanner: false,
        theme: lightTheme,
        darkTheme: darkTheme,
        themeMode: theme.mode,
        themeAnimationCurve: Curves.easeInOut,
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
                      const Offstage(child: Stack(children: [PushNotificationProvider(), InAppPurchaseProvider()])),
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
