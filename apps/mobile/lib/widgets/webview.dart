import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/logger.dart';
import 'package:typie/services/preference.dart';

class WebView extends StatefulHookWidget {
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
  late final WebViewController _controller;

  @override
  Widget build(BuildContext context) {
    final pref = useService<Pref>();
    final initialUri = Uri.parse(widget.initialUrl);

    final viewType = useMemoized(() {
      if (Platform.isAndroid && pref.androidGeckoView) {
        return 'co.typie.webview.gecko';
      }

      return 'co.typie.webview';
    }, [pref.androidGeckoView]);

    final creationParams = <String, dynamic>{
      'userAgent': widget.userAgent ?? 'Typie/1.0.0',
      'initialUrl': widget.initialUrl,
      'initialCookies': widget.initialCookies
          ?.map((cookie) => {'name': cookie.name, 'value': cookie.value, 'domain': initialUri.host})
          .toList(),
    };

    final child = switch (Platform.operatingSystem) {
      'android' => PlatformViewLink(
        viewType: viewType,
        surfaceFactory: (context, controller) {
          return AndroidViewSurface(
            controller: controller as AndroidViewController,
            gestureRecognizers: const <Factory<OneSequenceGestureRecognizer>>{},
            hitTestBehavior: PlatformViewHitTestBehavior.opaque,
          );
        },
        onCreatePlatformView: (params) {
          return PlatformViewsService.initSurfaceAndroidView(
              id: params.id,
              viewType: viewType,
              layoutDirection: TextDirection.ltr,
              creationParams: creationParams,
              creationParamsCodec: const StandardMessageCodec(),
              onFocus: () {
                params.onFocusChanged(true);
              },
            )
            ..addOnPlatformViewCreatedListener((id) {
              params.onPlatformViewCreated(id);
              _onPlatformViewCreated(viewType, id);
            })
            // ignore: discarded_futures for ease of use
            ..create();
        },
      ),
      'ios' => UiKitView(
        viewType: viewType,
        creationParams: creationParams,
        creationParamsCodec: const StandardMessageCodec(),
        onPlatformViewCreated: (id) => _onPlatformViewCreated(viewType, id),
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

  void _onPlatformViewCreated(String viewType, int id) {
    final channel = MethodChannel('$viewType.$id')
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
    await _channel.invokeMethod('emitEvent', {'name': name, 'data': jsonEncode(data)});
  }
}

class WebViewEvent {
  const WebViewEvent(this.name, [this.data]);

  final String name;
  final dynamic data;
}
