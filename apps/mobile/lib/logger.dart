import 'package:flutter/foundation.dart';
import 'package:logger/logger.dart';

class _Filter extends LogFilter {
  @override
  bool shouldLog(LogEvent event) {
    if (kDebugMode) {
      return true;
    }

    return event.level.value >= Level.warning.value;
  }
}

final log = Logger(filter: _Filter(), printer: SimplePrinter(colors: false));
