import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/service.dart';

T useService<T extends Object>() {
  return useMemoized(() => serviceLocator.call<T>());
}
