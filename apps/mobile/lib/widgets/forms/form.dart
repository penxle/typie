import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:luthor/luthor.dart';

class HookFormController extends ChangeNotifier {
  HookFormController({this.schema, this.onSubmit});

  final Validator? schema;
  final FutureOr<void> Function(HookFormController form)? onSubmit;

  final Map<String, dynamic> _data = {};
  final Map<String, String> _errors = {};

  bool _validated = false;

  Map<String, dynamic> get data => _data;
  Map<String, String> get errors => _errors;
  bool get isValid => !_validated || _errors.isEmpty;

  void setValue(String name, dynamic value) {
    _data[name] = value;
    notifyListeners();

    if (_validated) {
      _validate();
    }
  }

  void setError(String name, String error) {
    _errors[name] = error;
    notifyListeners();
  }

  Future<void> submit() async {
    _validate();

    if (isValid) {
      await onSubmit?.call(this);
    }
  }

  void _validate() {
    _validated = true;

    if (schema == null) {
      return;
    }

    final result = schema!.validateSchema<dynamic>(_data);
    if (result.isValid) {
      _errors.clear();
    } else if (result case SchemaValidationError(:final errors)) {
      for (final field in _data.entries) {
        final error = (errors[field.key] as List<dynamic>?)?.first as String?;
        if (error == null) {
          _errors.remove(field.key);
        } else {
          _errors[field.key] = error;
        }
      }
    }

    notifyListeners();
  }
}

class HookForm extends HookWidget {
  const HookForm({required this.builder, this.onSubmit, this.schema, super.key});

  final Widget Function(BuildContext context, HookFormController form) builder;
  final Validator? schema;
  final FutureOr<void> Function(HookFormController form)? onSubmit;

  @override
  Widget build(BuildContext context) {
    final controller = useMemoized(() => HookFormController(schema: schema, onSubmit: onSubmit));
    useListenableSelector(controller, () => controller.isValid);

    return HookFormScope(controller: controller, child: builder(context, controller));
  }
}

class HookFormScope extends InheritedWidget {
  const HookFormScope({required this.controller, required super.child, super.key});

  final HookFormController controller;

  static HookFormController of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<HookFormScope>();
    return scope!.controller;
  }

  @override
  bool updateShouldNotify(covariant HookFormScope old) => false;
}
