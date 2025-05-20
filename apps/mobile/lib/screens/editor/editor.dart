import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/env.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/editor/schema.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/webview.dart';

class Editor extends HookWidget {
  const Editor({required this.slug, super.key});

  final String slug;

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final keyboard = useService<Keyboard>();

    final isReady = useState(false);

    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);

    useEffect(() {
      final subscription = keyboard.onHeightChange.listen((height) {
        if (height > 0) {
          scope.keyboardHeight.value = height;
          scope.selectedToolboxIdx.value = -1;
        }

        scope.isKeyboardVisible.value = height > 0;
      });

      return subscription.cancel;
    }, [keyboard.onHeightChange]);

    useEffect(() {
      if (webViewController == null) {
        return null;
      }

      final subscription = webViewController.onEvent.listen((event) async {
        switch (event.name) {
          case 'webviewReady':
            isReady.value = true;
            await webViewController.requestFocus();
            await webViewController.emitEvent('appReady');
          case 'setProseMirrorState':
            scope.proseMirrorState.value = ProseMirrorState.fromJson(event.data as Map<String, dynamic>);
        }
      });

      return subscription.cancel;
    }, [webViewController]);

    if (auth.value case Authenticated(:final accessToken)) {
      return Screen(
        heading: const Heading(title: 'Editor'),
        safeArea: false,
        keyboardDismiss: false,
        child: Stack(
          fit: StackFit.expand,
          children: [
            Opacity(
              opacity: isReady.value ? 1 : 0.01,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(
                    child: WebView(
                      initialUrl: '${Env.websiteUrl}/_webview/editor?slug=$slug',
                      initialCookies: [Cookie('typie-at', accessToken)],
                      onWebViewCreated: (controller) {
                        scope.webViewController.value = controller;
                      },
                    ),
                  ),
                  const EditorToolbar(),
                ],
              ),
            ),
            if (!isReady.value) const Positioned.fill(child: Center(child: CircularProgressIndicator())),
          ],
        ),
      );
    }

    return const SizedBox.shrink();
  }
}
