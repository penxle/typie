import 'package:flutter/material.dart';
import 'package:typie/app.dart';
import 'package:typie/service.dart';
import 'package:typie/services/static.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  await configureStaticServices();
  await configureServices();

  runApp(const App());
}
