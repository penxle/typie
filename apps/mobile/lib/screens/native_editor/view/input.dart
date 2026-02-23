import 'dart:async';
import 'dart:io';

import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

class InputView extends StatefulWidget {
  const InputView({
    required this.brightness,
    required this.onInsertText,
    required this.onDeleteBackward,
    required this.onSetMarkedText,
    required this.onUnmarkText,
    required this.onCancelMarkedText,
    required this.onPerformAction,
    this.onReplaceBackward,
    this.onShortcut,
    this.onFloatingCursorBegin,
    this.onFloatingCursorUpdate,
    this.onFloatingCursorEnd,
    this.onFocusLost,
    this.onReady,
    this.onNavigate,
    super.key,
  });

  final Brightness brightness;
  final void Function(String text) onInsertText;
  final VoidCallback onDeleteBackward;
  final void Function(String text) onSetMarkedText;
  final VoidCallback onUnmarkText;
  final VoidCallback onCancelMarkedText;
  final void Function(int length, String text)? onReplaceBackward;
  final void Function(String action) onPerformAction;
  final void Function(String action)? onShortcut;
  final VoidCallback? onFloatingCursorBegin;
  final void Function(double dx, double dy)? onFloatingCursorUpdate;
  final VoidCallback? onFloatingCursorEnd;
  final VoidCallback? onFocusLost;
  final VoidCallback? onReady;
  final void Function(String direction, bool extend)? onNavigate;

  @override
  State<InputView> createState() => InputViewState();
}

class InputViewState extends State<InputView> {
  MethodChannel? _channel;
  Offset _cursorAnchor = Offset.zero;

  void _syncKeyboardAppearance() {
    if (!Platform.isIOS) {
      return;
    }

    final appearance = switch (widget.brightness) {
      Brightness.dark => 'dark',
      Brightness.light => 'light',
    };

    unawaited(_channel?.invokeMethod('setKeyboardAppearance', <String, dynamic>{'appearance': appearance}));
  }

  void activateInput() {
    unawaited(_channel?.invokeMethod('activate', <String, dynamic>{}));
  }

  void deactivateInput() {
    unawaited(_channel?.invokeMethod('deactivate', <String, dynamic>{}));
  }

  void resetInputContext() {
    unawaited(_channel?.invokeMethod('resetInputContext', <String, dynamic>{}));
  }

  void updateCursor(double x, double y, double height, [List<double>? precedingCharWidths]) {
    final nextAnchor = Offset(x, y);
    if (nextAnchor != _cursorAnchor) {
      setState(() {
        _cursorAnchor = nextAnchor;
      });
    }
    unawaited(
      _channel?.invokeMethod('updateCursor', <String, dynamic>{
        'x': 0,
        'y': 0,
        'height': height,
        'precedingCharWidths': precedingCharWidths,
      }),
    );
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
          case 'replaceBackward':
            widget.onReplaceBackward?.call(args!['length'] as int, args['text'] as String);
          case 'performAction':
            widget.onPerformAction(args!['action'] as String);
          case 'shortcut':
            widget.onShortcut?.call(args!['action'] as String);
          case 'floatingCursorBegin':
            widget.onFloatingCursorBegin?.call();
          case 'floatingCursorUpdate':
            widget.onFloatingCursorUpdate?.call(args!['dx'] as double, args['dy'] as double);
          case 'floatingCursorEnd':
            widget.onFloatingCursorEnd?.call();
          case 'focusLost':
            widget.onFocusLost?.call();
          case 'navigate':
            widget.onNavigate?.call(args!['direction'] as String, args['extend'] as bool);
          default:
            throw MissingPluginException('Method ${call.method} not implemented');
        }
      });
    _syncKeyboardAppearance();
    widget.onReady?.call();
  }

  @override
  void didUpdateWidget(covariant InputView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.brightness != widget.brightness) {
      _syncKeyboardAppearance();
    }
  }

  @override
  void dispose() {
    _channel?.setMethodCallHandler(null);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    const viewType = 'co.typie.editor_input';

    final platformView = Platform.isIOS
        ? UiKitView(
            viewType: viewType,
            hitTestBehavior: PlatformViewHitTestBehavior.transparent,
            onPlatformViewCreated: _onPlatformViewCreated,
          )
        : AndroidView(
            viewType: viewType,
            hitTestBehavior: PlatformViewHitTestBehavior.transparent,
            onPlatformViewCreated: _onPlatformViewCreated,
          );

    return Stack(
      clipBehavior: Clip.none,
      children: [Positioned(left: _cursorAnchor.dx, top: _cursorAnchor.dy, width: 1, height: 1, child: platformView)],
    );
  }
}
