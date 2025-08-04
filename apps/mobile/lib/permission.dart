import 'dart:io';

import 'package:permission_handler/permission_handler.dart';

Future<void> requestPermissions() async {
  if (Platform.isIOS) {
    await Permission.appTrackingTransparency.request();
  }
}
