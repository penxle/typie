import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:luthor/luthor.dart';

class HookFormController extends ChangeNotifier {
  HookFormController({this.schema, this.onSubmit, this.submitMode = HookFormSubmitMode.onSubmit});

  final Validator? schema;
  final Future<void>? Function(HookFormController form)? onSubmit;
  final HookFormSubmitMode submitMode;

  final Map<String, dynamic> _data = {};
  final Map<String, String> _errors = {};

  var _validated = false;

  Map<String, dynamic> get data => _data;
  Map<String, String> get errors => _errors;
  bool get isValid => !_validated || _errors.isEmpty;

  void setValue(String name, dynamic value, {bool notify = true}) {
    _data[name] = value;

    if (!notify) {
      return;
    }

    notifyListeners();

    if (_validated && submitMode == HookFormSubmitMode.onSubmit) {
      _validate();
    } else if (submitMode == HookFormSubmitMode.onChange) {
      unawaited(submit());
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
    if (result case SchemaValidationSuccess(:final data)) {
      for (final field in (data as Map<String, dynamic>).entries) {
        _data[field.key] = field.value;
        _errors.remove(field.key);
      }
    } else if (result case SchemaValidationError(:final data, :final errors)) {
      for (final field in (data as Map<String, dynamic>).entries) {
        _data[field.key] = field.value;
        _errors.remove(field.key);
      }

      for (final field in errors.entries) {
        final error = (errors[field.key] as List<dynamic>).first as String;
        _errors[field.key] = error;
      }
    }

    notifyListeners();
  }
}

enum HookFormSubmitMode { onSubmit, onChange }

class HookForm extends HookWidget {
  const HookForm({
    required this.builder,
    this.onSubmit,
    this.schema,
    this.submitMode = HookFormSubmitMode.onSubmit,
    super.key,
  });

  final Widget Function(BuildContext context, HookFormController form) builder;
  final HookFormSubmitMode submitMode;
  final Validator? schema;
  final Future<void>? Function(HookFormController form)? onSubmit;

  @override
  Widget build(BuildContext context) {
    final controller = useMemoized(
      () => HookFormController(schema: schema, onSubmit: onSubmit, submitMode: submitMode),
    );

    useListenable(controller);

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
