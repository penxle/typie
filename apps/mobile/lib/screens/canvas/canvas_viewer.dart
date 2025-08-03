import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/env.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/canvas/scope.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/services/theme.dart';
import 'package:typie/widgets/webview.dart';

class CanvasViewer extends HookWidget {
  const CanvasViewer({super.key, required this.siteId, required this.slug});

  final String siteId;
  final String slug;

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final theme = useService<AppTheme>();

    final isReady = useState(false);

    final scope = CanvasViewerStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);

    useEffect(() {
      if (webViewController == null) {
        return null;
      }

      final subscription = webViewController.onEvent.listen((event) {
        if (event.name == 'webviewReady') {
          isReady.value = true;
        }
      });

      return subscription.cancel;
    }, [webViewController]);

    return Stack(
      children: [
        Opacity(
          opacity: isReady.value ? 1 : 0.01,
          child: WebView(
            initialUrl: '${Env.websiteUrl}/_webview/canvas?siteId=$siteId&slug=$slug',
            initialCookies: [
              Cookie('typie-at', (auth.value as Authenticated).accessToken),
              Cookie('typie-th', switch (theme.mode) {
                ThemeMode.system => 'auto',
                ThemeMode.light => 'light',
                ThemeMode.dark => 'dark',
              }),
            ],
            onWebViewCreated: (controller) {
              scope.webViewController.value = controller;
            },
          ),
        ),
        if (!isReady.value) const Center(child: CircularProgressIndicator()),
        if (isReady.value)
          Positioned(
            left: 40,
            bottom: 40,
            child: GestureDetector(
              onTap: () {
                context.toast(ToastType.notification, '아직 캔버스는 앱에서 편집할 수 없어요', bottom: 64);
              },
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                decoration: BoxDecoration(
                  color: context.colors.surfaceDefault.withValues(alpha: 0.95),
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(color: context.colors.borderDefault),
                  boxShadow: [
                    BoxShadow(color: Colors.black.withValues(alpha: 0.1), blurRadius: 6, offset: const Offset(0, 2)),
                  ],
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  spacing: 5,
                  children: [
                    Icon(Icons.visibility_outlined, size: 16, color: context.colors.textSubtle),
                    Text(
                      '보기 전용',
                      style: TextStyle(fontSize: 13, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
                    ),
                  ],
                ),
              ),
            ),
          ),
      ],
    );
  }
}
