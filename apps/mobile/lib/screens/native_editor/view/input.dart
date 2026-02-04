import 'dart:async';
import 'dart:io';

import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

class InputView extends StatefulWidget {
  const InputView({
    required this.onInsertText,
    required this.onDeleteBackward,
    required this.onSetMarkedText,
    required this.onUnmarkText,
    required this.onCancelMarkedText,
    required this.onPerformAction,
    this.onShortcut,
    this.onFocusLost,
    this.onReady,
    super.key,
  });

  final void Function(String text) onInsertText;
  final VoidCallback onDeleteBackward;
  final void Function(String text) onSetMarkedText;
  final VoidCallback onUnmarkText;
  final VoidCallback onCancelMarkedText;
  final void Function(String action) onPerformAction;
  final void Function(String action)? onShortcut;
  final VoidCallback? onFocusLost;
  final VoidCallback? onReady;

  @override
  State<InputView> createState() => InputViewState();
}

class InputViewState extends State<InputView> {
  MethodChannel? _channel;

  void activateInput() {
    unawaited(_channel?.invokeMethod('activate', <String, dynamic>{}));
  }

  void deactivateInput() {
    unawaited(_channel?.invokeMethod('deactivate', <String, dynamic>{}));
  }

  void resetInputContext() {
    unawaited(_channel?.invokeMethod('resetInputContext', <String, dynamic>{}));
  }

  void updateCursor(double x, double y, double height) {
    unawaited(_channel?.invokeMethod('updateCursor', <String, dynamic>{'x': x, 'y': y, 'height': height}));
  }

  void _onPlatformViewCreated(int id) {
    _channel = MethodChannel('co.typie.editor_input.$id')
      ..setMethodCallHandler((call) async {
        final args = call.arguments as Map<dynamic, dynamic>?;
        switch (call.method) {
          case 'insertText':
            widget.onInsertText(args!['text'] as String);
          case 'deleteBackward':
            widget.onDeleteBackward();
          case 'setMarkedText':
            widget.onSetMarkedText(args!['text'] as String);
          case 'unmarkText':
            widget.onUnmarkText();
          case 'cancelMarkedText':
            widget.onCancelMarkedText();
          case 'performAction':
            widget.onPerformAction(args!['action'] as String);
          case 'shortcut':
            widget.onShortcut?.call(args!['action'] as String);
          case 'focusLost':
            widget.onFocusLost?.call();
          default:
            throw MissingPluginException('Method ${call.method} not implemented');
        }
      });
    widget.onReady?.call();
  }

  @override
  void dispose() {
    _channel?.setMethodCallHandler(null);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    const viewType = 'co.typie.editor_input';

    if (Platform.isIOS) {
      return UiKitView(
        viewType: viewType,
        hitTestBehavior: PlatformViewHitTestBehavior.transparent,
        onPlatformViewCreated: _onPlatformViewCreated,
      );
    }

    return AndroidView(
      viewType: viewType,
      hitTestBehavior: PlatformViewHitTestBehavior.transparent,
      onPlatformViewCreated: _onPlatformViewCreated,
    );
  }
}
