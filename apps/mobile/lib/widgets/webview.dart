import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:typie/logger.dart';

class WebView extends StatefulWidget {
  const WebView({required this.initialUrl, this.userAgent, this.onWebViewCreated, super.key});

  final String initialUrl;
  final String? userAgent;
  final void Function(WebViewController controller)? onWebViewCreated;

  @override
  State<WebView> createState() {
    return _WebViewState();
  }
}

class _WebViewState extends State<WebView> {
  late final WebViewController _controller;

  @override
  Widget build(BuildContext context) {
    final params = <String, dynamic>{'initialUrl': widget.initialUrl, 'userAgent': widget.userAgent ?? 'Typie/1.0.0'};

    if (Platform.isAndroid) {
      return AndroidView(
        viewType: 'co.typie.webview',
        creationParams: params,
        creationParamsCodec: const StandardMessageCodec(),
        onPlatformViewCreated: _onPlatformViewCreated,
      );
    } else if (Platform.isIOS) {
      return UiKitView(
        viewType: 'co.typie.webview',
        creationParams: params,
        creationParamsCodec: const StandardMessageCodec(),
        onPlatformViewCreated: _onPlatformViewCreated,
      );
    }

    throw UnimplementedError('WebView is not supported on ${Platform.operatingSystem}');
  }

  @override
  void dispose() {
    _controller._channel.invokeMethod('dispose', <dynamic, dynamic>{});

    super.dispose();
  }

  void _onPlatformViewCreated(int id) {
    final channel = MethodChannel('co.typie.webview.$id')..setMethodCallHandler((call) async {
      try {
        final args = call.arguments as Map<dynamic, dynamic>;
        switch (call.method) {
          case 'onConsole':
            _onConsole(args['level'] as String, args['message'] as String);
          default:
            throw MissingPluginException('Method ${call.method} not implemented');
        }
      } on MissingPluginException {
        rethrow;
        // ignore: avoid_catches_without_on_clauses catch all
      } catch (e) {
        log.e('WebView', error: e);
      }
    });

    _controller = WebViewController(channel);
    widget.onWebViewCreated?.call(_controller);
  }

  void _onConsole(String level, String message) {
    log.d('WebView: [$level] $message');
  }
}

class WebViewController {
  WebViewController(this._channel);

  final MethodChannel _channel;

  Future<void> loadUrl(String url) async {
    await _channel.invokeMethod('loadUrl', {'url': url});
  }
}
