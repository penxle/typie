import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:typie/logger.dart';

class WebView extends StatefulWidget {
  const WebView({required this.initialUrl, this.initialCookies, this.userAgent, this.onWebViewCreated, super.key});

  final String initialUrl;
  final List<Cookie>? initialCookies;
  final String? userAgent;
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
    final initialUri = Uri.parse(widget.initialUrl);

    final creationParams = <String, dynamic>{
      'userAgent': widget.userAgent ?? 'Typie/1.0.0',
      'initialUrl': widget.initialUrl,
      'initialCookies': widget.initialCookies
          ?.map((cookie) => {'name': cookie.name, 'value': cookie.value, 'domain': initialUri.host})
          .toList(),
    };

    if (Platform.isAndroid) {
      return AndroidView(
        viewType: viewType,
        creationParams: creationParams,
        creationParamsCodec: const StandardMessageCodec(),
        onPlatformViewCreated: _onPlatformViewCreated,
      );
    } else if (Platform.isIOS) {
      return UiKitView(
        viewType: viewType,
        creationParams: creationParams,
        creationParamsCodec: const StandardMessageCodec(),
        onPlatformViewCreated: _onPlatformViewCreated,
      );
    }

    throw UnimplementedError('WebView is not supported on ${Platform.operatingSystem}');
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
