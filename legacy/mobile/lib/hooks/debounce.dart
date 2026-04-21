import 'dart:async';

import 'package:flutter_hooks/flutter_hooks.dart';

class Debounce<T> {
  Debounce(this.call, this.cancel, this.timers);

  final void Function(T Function() fn, [String? id]) call;
  final void Function([String? id]) cancel;
  final Map<String, Timer> Function() timers;
}

Debounce<T> useDebounce<T>(Duration duration) {
  final timers = useRef<Map<String, Timer>>({});

  useEffect(() {
    return () {
      for (final timer in timers.value.values) {
        timer.cancel();
      }
      timers.value.clear();
    };
  }, []);

  void call(T Function() fn, [String? id]) {
    final key = id ?? 'default';
    timers.value[key]?.cancel();
    timers.value[key] = Timer(duration, () {
      fn();
      timers.value.remove(key);
    });
  }

  void cancel([String? id]) {
    final key = id ?? 'default';
    timers.value[key]?.cancel();
    timers.value.remove(key);
  }

  Map<String, Timer> getTimers() {
    return timers.value;
  }

  return Debounce(call, cancel, getTimers);
}
