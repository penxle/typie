import 'dart:async';

import 'package:flutter/services.dart';
import 'package:injectable/injectable.dart';

@singleton
class Keyboard {
  Keyboard() {
    _channel.receiveBroadcastStream().listen((event) {
      final data = event as Map<dynamic, dynamic>;
      _height = data['height'] as double;
      _streamController.add(_height);
    });
  }

  static const _channel = EventChannel('co.typie.keyboard');
  final _streamController = StreamController<double>.broadcast();
  double _height = 0;

  Stream<double> get onHeightChange => _streamController.stream;
  double get height => _height;
  bool get isVisible => _height > 0;

  @disposeMethod
  Future<void> dispose() async {
    await _streamController.close();
  }
}
