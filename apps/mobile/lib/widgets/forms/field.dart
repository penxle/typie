import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/widgets/forms/form.dart';

class HookFormFieldController<T> {
  HookFormFieldController({required this.form, required this.name});

  final HookFormController form;
  final String name;

  T? get value => form.data[name] as T?;
  set value(T? value) => form.setValue(name, value);

  String? get error => form.errors[name];
}

class HookFormField<T> extends HookWidget {
  const HookFormField({required this.builder, required this.name, this.initialValue, super.key});

  final String name;
  final T? initialValue;
  final Widget Function(BuildContext context, HookFormFieldController<T> field) builder;

  @override
  Widget build(BuildContext context) {
    final form = HookFormScope.of(context);
    final field = useMemoized(() => HookFormFieldController<T>(form: form, name: name));

    useListenableSelector(form, () => form.data[name]);
    useListenableSelector(form, () => form.errors[name]);

    useEffect(() {
      if (initialValue != null) {
        form.setValue(name, initialValue, notify: false);
      }

      return null;
    }, []);

    return builder(context, field);
  }
}
