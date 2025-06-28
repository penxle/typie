import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/logger.dart';
import 'package:typie/services/theme.dart';

class WebView extends StatefulHookWidget {
  const WebView({required this.initialUrl, this.initialCookies, this.onWebViewCreated, super.key});

  final String initialUrl;
  final List<Cookie>? initialCookies;
  final void Function(WebViewController controller)? onWebViewCreated;

  @override
  State<WebView> createState() {
    return _WebViewState();
  }
}

class _WebViewState extends State<WebView> {
  final viewType = 'co.typie.webview';
  late final WebViewController _controller;

  @override
  Widget build(BuildContext context) {
    final theme = useService<AppTheme>();
    final initialUri = Uri.parse(widget.initialUrl);

    final userAgent = switch (Platform.operatingSystem) {
      'android' =>
        'Mozilla/5.0 (Linux; Android 16) AppleWebKit/600.0.00 (KHTML, like Gecko) Chrome/140.0.0.0 Mobile Safari/600.0 Typie/1.0.0',
      'ios' =>
        'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0_0 like Mac OS X) AppleWebKit/600.0.00 (KHTML, like Gecko) Version/18.0 Mobile/10A000 Safari/600.0 Typie/1.0.0',
      _ => throw UnimplementedError('WebView is not supported on ${Platform.operatingSystem}'),
    };

    final creationParams = <String, dynamic>{
      'themeMode': theme.mode.name,
      'userAgent': userAgent,
      'initialUrl': widget.initialUrl,
      'initialCookies': widget.initialCookies
          ?.map((cookie) => {'name': cookie.name, 'value': cookie.value, 'domain': initialUri.host})
          .toList(),
    };

    final child = switch (Platform.operatingSystem) {
      'android' => AndroidView(
        viewType: viewType,
        creationParams: creationParams,
        creationParamsCodec: const StandardMessageCodec(),
        onPlatformViewCreated: _onPlatformViewCreated,
      ),
      'ios' => UiKitView(
        viewType: viewType,
        creationParams: creationParams,
        creationParamsCodec: const StandardMessageCodec(),
        onPlatformViewCreated: _onPlatformViewCreated,
      ),
      _ => throw UnimplementedError('WebView is not supported on ${Platform.operatingSystem}'),
    };

    return Focus(
      onKeyEvent: (node, event) {
        return KeyEventResult.skipRemainingHandlers;
      },
      child: child,
    );
  }

  @override
  void dispose() {
    _controller._channel.setMethodCallHandler(null);

    unawaited(_controller._channel.invokeMethod('dispose', <dynamic, dynamic>{}));
    unawaited(_controller._streamController.close());

    super.dispose();
  }

  void _onPlatformViewCreated(int id) {
    final channel = MethodChannel('co.typie.webview.$id')
      ..setMethodCallHandler((call) async {
        try {
          final args = call.arguments as Map<dynamic, dynamic>;
          switch (call.method) {
            case 'console':
              _onConsole(args['level'] as String, args['message'] as String);
            case 'emitEvent':
              _onEmitEvent(args['name'] as String, jsonDecode(args['data'] as String));
            default:
              throw MissingPluginException('Method ${call.method} not implemented');
          }
        } on MissingPluginException {
          rethrow;
        } catch (err) {
          log.e('WebView', error: err);
        }
      });

    _controller = WebViewController(channel);
    widget.onWebViewCreated?.call(_controller);
  }

  void _onConsole(String level, String message) {
    log.d('WebView: [$level] $message');
  }

  void _onEmitEvent(String name, dynamic data) {
    _controller._streamController.add(WebViewEvent(name, data));
  }
}

class WebViewController {
  WebViewController(this._channel);

  final MethodChannel _channel;
  final _streamController = StreamController<WebViewEvent>.broadcast();

  Stream<WebViewEvent> get onEvent => _streamController.stream;

  Future<void> requestFocus() async {
    await _channel.invokeMethod('requestFocus', <dynamic, dynamic>{});
  }

  Future<void> clearFocus() async {
    await _channel.invokeMethod('clearFocus', <dynamic, dynamic>{});
  }

  Future<void> emitEvent(String name, [dynamic data]) async {
    final jsonData = jsonEncode(data)
        .replaceAll(r'\', r'\\')
        .replaceAll('"', r'\"')
        .replaceAll('\n', r'\n')
        .replaceAll('\r', r'\r')
        .replaceAll('\t', r'\t');

    await _channel.invokeMethod('emitEvent', {'name': name, 'data': jsonData});
  }
}

class WebViewEvent {
  const WebViewEvent(this.name, [this.data]);

  final String name;
  final dynamic data;
}
