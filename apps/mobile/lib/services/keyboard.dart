import 'dart:async';

import 'package:flutter/services.dart';
import 'package:injectable/injectable.dart';

enum KeyboardType { software, hardware }

@singleton
class Keyboard {
  Keyboard() {
    _subscription = _channel.receiveBroadcastStream().listen((event) {
      final data = event as Map<dynamic, dynamic>;

      switch (data['type']) {
        case 'height':
          _height = data['height'] as double;
          _heightStreamController.add(_height);
        case 'hardware':
          _type = data['hardware'] as bool ? KeyboardType.hardware : KeyboardType.software;
          _typeStreamController.add(_type);
      }
    });
  }

  static const _channel = EventChannel('co.typie.keyboard');
  late final StreamSubscription<dynamic> _subscription;

  final _heightStreamController = StreamController<double>.broadcast();
  final _typeStreamController = StreamController<KeyboardType>.broadcast();

  var _height = 0.0;
  KeyboardType _type = KeyboardType.software;

  Stream<double> get onHeightChange async* {
    yield _height;
    yield* _heightStreamController.stream;
  }

  Stream<KeyboardType> get onTypeChange async* {
    yield _type;
    yield* _typeStreamController.stream;
  }

  @disposeMethod
  Future<void> dispose() async {
    await _subscription.cancel();
    await _heightStreamController.close();
    await _typeStreamController.close();
  }
}
