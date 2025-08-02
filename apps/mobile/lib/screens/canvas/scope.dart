import 'package:flutter/material.dart';
import 'package:typie/widgets/webview.dart';

class CanvasViewerStateScope extends InheritedWidget {
  const CanvasViewerStateScope({required super.child, required this.webViewController, super.key});

  final ValueNotifier<WebViewController?> webViewController;

  static CanvasViewerStateScope of(BuildContext context) {
    final scope = context.getInheritedWidgetOfExactType<CanvasViewerStateScope>();
    return scope!;
  }

  @override
  bool updateShouldNotify(covariant CanvasViewerStateScope old) => false;
}
