import 'dart:async';
import 'dart:io';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:typie/app.dart';
import 'package:typie/instrument.dart';
import 'package:typie/permission.dart';
import 'package:typie/service.dart';
import 'package:typie/services/static.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  if (Platform.isIOS) {
    _installZeroOffsetPointerGuard();
  }

  await configureInstruments();
  await configureStaticServices();
  await configureServices();

  unawaited(requestPermissions());

  runApp(const App());
}

// Workaround for https://github.com/flutter/flutter/issues/175606
// iPadOS 26 sends fake touch events at Offset.zero when tapping near screen edges
void _installZeroOffsetPointerGuard() {
  GestureBinding.instance.pointerRouter.addGlobalRoute((event) {
    if (event.position == Offset.zero) {
      GestureBinding.instance.cancelPointer(event.pointer);
    }
  });
}
