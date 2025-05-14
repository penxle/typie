import 'dart:io';

import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/env.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/editor/toolbar.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/webview.dart';

@RoutePage()
class EditorScreen extends HookWidget {
  const EditorScreen({required this.slug, super.key});

  final String slug;

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();

    final webViewController = useState<WebViewController?>(null);
    final isReady = useState(false);

    final focusNode = useFocusNode();

    useEffect(() {
      if (webViewController.value == null) {
        return null;
      }

      final controller = webViewController.value!;

      final subscription = controller.onEvent.listen((event) {
        switch (event.name) {
          case 'ready':
            isReady.value = true;
            focusNode.requestFocus();
          case 'blur':
            focusNode.unfocus();
        }
      });

      return subscription.cancel;
    }, [webViewController.value]);

    if (auth.value case Authenticated(:final accessToken)) {
      return Screen(
        heading: const Heading(title: 'Editor'),
        safeArea: false,
        resizeToAvoidBottomInset: false,
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
                      focusNode: focusNode,
                      initialUrl: '${Env.websiteUrl}/_webview/editor?slug=$slug',
                      initialCookies: [Cookie('typie-at', accessToken)],
                      onWebViewCreated: (controller) {
                        webViewController.value = controller;
                      },
                    ),
                  ),
                  EditorToolbar(webViewController: webViewController.value),
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
