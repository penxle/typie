import 'dart:io';

import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/webview.dart';

@RoutePage()
class EditorScreen extends HookWidget {
  const EditorScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final webViewController = useRef<WebViewController?>(null);

    if (auth.value case Authenticated(:final accessToken)) {
      return Screen(
        heading: const Heading(title: 'Editor'),
        resizeToAvoidBottomInset: false,
        child: WebView(
          initialUrl: 'https://typie.dev',
          initialCookies: [Cookie('typie-at', accessToken)],
          onWebViewCreated: (controller) {
            webViewController.value = controller;
          },
        ),
      );
    }

    return const SizedBox.shrink();
  }
}
