import 'dart:io';

import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/env.dart';
import 'package:typie/hooks/service.dart';
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
    final webViewController = useRef<WebViewController?>(null);

    final focusNode = useFocusNode();

    if (auth.value case Authenticated(:final accessToken)) {
      return Screen(
        heading: const Heading(title: 'Editor'),
        resizeToAvoidBottomInset: false,
        child: WebView(
          focusNode: focusNode,
          initialUrl: '${Env.websiteUrl}/_webview/editor?slug=$slug',
          initialCookies: [Cookie('typie-at', accessToken)],
          onWebViewCreated: (controller) {
            webViewController.value = controller;
            focusNode.requestFocus();
          },
        ),
      );
    }

    return const SizedBox.shrink();
  }
}
