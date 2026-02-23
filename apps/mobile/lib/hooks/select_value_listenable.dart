import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';

T useSelectValueListenable<S, T>(
  ValueListenable<S> listenable,
  T Function(S value) selector, {
  bool Function(T previous, T next)? equals,
}) {
  return use(_SelectValueListenableHook<S, T>(listenable, selector, equals));
}

const _defaultEquality = DeepCollectionEquality();

class _SelectValueListenableHook<S, T> extends Hook<T> {
  const _SelectValueListenableHook(this.listenable, this.selector, this.equals);

  final ValueListenable<S> listenable;
  final T Function(S value) selector;
  final bool Function(T previous, T next)? equals;

  @override
  _SelectValueListenableHookState<S, T> createState() => _SelectValueListenableHookState();
}

class _SelectValueListenableHookState<S, T> extends HookState<T, _SelectValueListenableHook<S, T>> {
  late T _selectedValue;

  @override
  void initHook() {
    super.initHook();
    _selectedValue = hook.selector(hook.listenable.value);
    hook.listenable.addListener(_listener);
  }

  @override
  void didUpdateHook(_SelectValueListenableHook<S, T> oldHook) {
    super.didUpdateHook(oldHook);
    if (oldHook.listenable != hook.listenable) {
      oldHook.listenable.removeListener(_listener);
      hook.listenable.addListener(_listener);
      _selectedValue = hook.selector(hook.listenable.value);
    }
  }

  void _listener() {
    final newValue = hook.selector(hook.listenable.value);
    final isEqual = hook.equals != null
        ? hook.equals!(_selectedValue, newValue)
        : _defaultEquality.equals(_selectedValue, newValue);
    if (!isEqual) {
      _selectedValue = newValue;
      setState(() {});
    }
  }

  @override
  T build(BuildContext context) => _selectedValue;

  @override
  void dispose() {
    hook.listenable.removeListener(_listener);
    super.dispose();
  }

  @override
  String get debugLabel => 'useSelectValueListenable';
}
