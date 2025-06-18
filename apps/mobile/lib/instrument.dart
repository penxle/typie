import 'package:flutter/foundation.dart';
import 'package:sentry_flutter/sentry_flutter.dart';
import 'package:typie/env.dart';

Future<void> configureInstruments() async {
  if (kDebugMode) {
    return;
  }

  await SentryFlutter.init((options) {
    options
      ..dsn = Env.sentryDsn
      ..attachScreenshot = true
      ..sendDefaultPii = true
      ..privacy.maskAllText = false
      ..privacy.maskAllImages = false
      ..replay.onErrorSampleRate = 1.0
      ..replay.sessionSampleRate = 0.1
      ..replay.quality = SentryReplayQuality.high;
  });
}
