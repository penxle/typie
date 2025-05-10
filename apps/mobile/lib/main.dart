import 'package:firebase_core/firebase_core.dart';
import 'package:flutter/material.dart';
import 'package:typie/app.dart';
import 'package:typie/firebase/options.dart';
import 'package:typie/service.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  await Firebase.initializeApp(options: DefaultFirebaseOptions.currentPlatform);

  await configureServices();

  runApp(const App());
}
