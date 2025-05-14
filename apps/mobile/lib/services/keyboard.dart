import 'dart:async';

import 'package:flutter/services.dart';
import 'package:injectable/injectable.dart';

@singleton
class Keyboard {
  Keyboard() {
    _channel
      ..setMethodCallHandler((call) async {
        final args = call.arguments as Map<dynamic, dynamic>;
        if (call.method == 'heightChanged') {
          _height = args['height'] as double;
          _streamController.add(_height);
        }
      })
      ..invokeMethod('listen');
  }

  static const _channel = MethodChannel('co.typie.keyboard');
  final _streamController = StreamController<double>.broadcast();
  double _height = 0;

  Stream<double> get onHeightChange => _streamController.stream;
  double get height => _height;
  bool get isVisible => _height > 0;

  @disposeMethod
  void dispose() {
    _streamController.close();
    _channel.invokeMethod('dispose');
  }
}
