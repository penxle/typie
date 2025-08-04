import 'dart:async';

import 'package:flutter/material.dart';
import 'package:typie/app.dart';
import 'package:typie/instrument.dart';
import 'package:typie/permission.dart';
import 'package:typie/service.dart';
import 'package:typie/services/static.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  await configureInstruments();
  await configureStaticServices();
  await configureServices();

  unawaited(requestPermissions());

  runApp(const App());
}
