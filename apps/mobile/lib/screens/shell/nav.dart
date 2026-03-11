import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';

class ShellTrailingActionConfig {
  const ShellTrailingActionConfig({required this.icon, this.onTap, this.pane}) : assert(onTap != null || pane != null);

  final IconData icon;
  final Future<void> Function()? onTap;
  final Widget? pane;
}

class ShellTrailingActionController extends ValueNotifier<ShellTrailingActionConfig?> {
  ShellTrailingActionController() : super(null);

  final _entries = <Object, ShellTrailingActionConfig>{};
  int _version = 0;

  void setFor(Object owner, ShellTrailingActionConfig config) {
    _entries.remove(owner);
    _entries[owner] = config;
    _version += 1;
    _publish(_entries.isEmpty ? null : _entries.values.last, _version);
  }

  void clearFor(Object owner) {
    if (_entries.remove(owner) == null) {
      return;
    }

    _version += 1;
    _publish(_entries.isEmpty ? null : _entries.values.last, _version);
  }

  void _publish(ShellTrailingActionConfig? nextValue, int version) {
    if (value == nextValue) {
      return;
    }

    final phase = SchedulerBinding.instance.schedulerPhase;
    if (phase == SchedulerPhase.idle || phase == SchedulerPhase.postFrameCallbacks) {
      value = nextValue;
      return;
    }

    SchedulerBinding.instance.addPostFrameCallback((_) {
      if (_version != version) {
        return;
      }

      if (value != nextValue) {
        value = nextValue;
      }
    });
  }
}

class ShellNav extends InheritedWidget {
  const ShellNav({required this.visible, required this.trailingAction, required super.child, super.key});

  final ValueNotifier<bool> visible;
  final ShellTrailingActionController trailingAction;

  static ShellNav? maybeOf(BuildContext context) => context.dependOnInheritedWidgetOfExactType<ShellNav>();
  static ShellNav of(BuildContext context) => maybeOf(context)!;

  void hide() => visible.value = false;
  void show() => visible.value = true;
  void setTrailingAction(Object owner, ShellTrailingActionConfig config) => trailingAction.setFor(owner, config);
  void clearTrailingAction(Object owner) => trailingAction.clearFor(owner);

  @override
  bool updateShouldNotify(ShellNav oldWidget) {
    return visible != oldWidget.visible || trailingAction != oldWidget.trailingAction;
  }
}
