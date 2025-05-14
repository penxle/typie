import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:typie/logger.dart';

class WebView extends StatefulWidget {
  const WebView({
    required this.initialUrl,
    this.initialCookies,
    this.userAgent,
    this.onWebViewCreated,
    this.focusNode,
    super.key,
  });

  final String initialUrl;
  final List<Cookie>? initialCookies;
  final String? userAgent;
  final void Function(WebViewController controller)? onWebViewCreated;

  final FocusNode? focusNode;

  @override
  State<WebView> createState() {
    return _WebViewState();
  }
}

class _WebViewState extends State<WebView> {
  final viewType = 'co.typie.webview';
  late final WebViewController _controller;

  @override
  void initState() {
    super.initState();

    if (widget.focusNode != null) {
      widget.focusNode!.addListener(_onFocusChanged);
    }
  }

  @override
  Widget build(BuildContext context) {
    final initialUri = Uri.parse(widget.initialUrl);

    final creationParams = <String, dynamic>{
      'userAgent': widget.userAgent ?? 'Typie/1.0.0',
      'initialUrl': widget.initialUrl,
      'initialCookies':
          widget.initialCookies
              ?.map((cookie) => {'name': cookie.name, 'value': cookie.value, 'domain': initialUri.host})
              .toList(),
    };

    if (Platform.isAndroid) {
      return Focus(
        focusNode: widget.focusNode,
        child: AndroidView(
          viewType: viewType,
          creationParams: creationParams,
          creationParamsCodec: const StandardMessageCodec(),
          onPlatformViewCreated: _onPlatformViewCreated,
        ),
      );
    } else if (Platform.isIOS) {
      return Focus(
        focusNode: widget.focusNode,
        child: UiKitView(
          viewType: viewType,
          creationParams: creationParams,
          creationParamsCodec: const StandardMessageCodec(),
          onPlatformViewCreated: _onPlatformViewCreated,
        ),
      );
    }

    throw UnimplementedError('WebView is not supported on ${Platform.operatingSystem}');
  }

  @override
  void dispose() {
    if (widget.focusNode != null) {
      widget.focusNode!.removeListener(_onFocusChanged);
    }

    _controller._channel.invokeMethod('dispose', <dynamic, dynamic>{});
    _controller._channel.setMethodCallHandler(null);
    _controller._streamController.close();

    super.dispose();
  }

  void _onPlatformViewCreated(int id) {
    final channel = MethodChannel('co.typie.webview.$id')..setMethodCallHandler((call) async {
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

  void _onEmitEvent(String name, dynamic data) {
    _controller._streamController.add(WebViewEvent(name, data));
  }

  void _onFocusChanged() {
    if (widget.focusNode!.hasFocus) {
      _controller.requestFocus();
    }
  }
}

class WebViewController {
  WebViewController(this._channel);

  final MethodChannel _channel;
  final _streamController = StreamController<WebViewEvent>.broadcast();

  Stream<WebViewEvent> get onEvent => _streamController.stream;

  Future<void> loadUrl(String url) async {
    await _channel.invokeMethod('loadUrl', {'url': url});
  }

  Future<void> requestFocus() async {
    await _channel.invokeMethod('requestFocus', <dynamic, dynamic>{});
  }

  void emitEvent(String name, [dynamic data]) {
    _channel.invokeMethod('emitEvent', {'name': name, 'data': jsonEncode(data)});
  }
}

class WebViewEvent {
  const WebViewEvent(this.name, [this.data]);

  final String name;
  final dynamic data;
}
