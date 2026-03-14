import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/hooks/select_value_listenable.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/label.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';

const minFontSize = 100;
const maxFontSize = 20000;

class NativeEditorFontSizeTextOptionsToolbar extends HookWidget {
  const NativeEditorFontSizeTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final fontSizeAttr = useSelectValueListenable(scope.attrs, (attrs) => findAttr(attrs, 'font_size'));
    final fontSizeValues = (fontSizeAttr?['values'] as List?)?.whereType<num>().toList() ?? [];
    final activeValue = fontSizeValues.length == 1
        ? fontSizeValues[0]
        : (fontSizeValues.isEmpty ? editorDefaultValues['fontSize'] as num : null);

    final presetItems = editorValues['fontSize']!;
    final allItems = useMemoized(() {
      final items = List<Map<String, dynamic>>.from(presetItems);
      if (activeValue == null) {
        return items;
      }
      final num activeNum = activeValue;

      if (!items.any((item) => item['value'] == activeNum)) {
        var insertIndex = items.indexWhere((item) => (item['value'] as num) > activeNum);
        if (insertIndex == -1) {
          insertIndex = items.length;
        }
        final displayValue = activeNum / 100;
        final label = displayValue % 1 == 0 ? displayValue.toInt().toString() : displayValue.toString();
        items.insert(insertIndex, {'label': label, 'value': activeNum});
      }

      return items;
    }, [activeValue]);

    return NativeEditorTextOptionsToolbar(
      items: allItems,
      activeValue: activeValue,
      builder: (context, item, isActive) {
        return LabelToolbarButton(
          text: item['label'] as String,
          isActive: isActive,
          suffix: isActive ? const Icon(LucideLightIcons.pencil, size: 14) : null,
          prepareMutationOnTapDown: true,
          onTap: () async {
            if (isActive) {
              await context.showModal(
                intercept: true,
                child: HookForm(
                  onSubmit: (form) async {
                    final parsed = num.tryParse(form.data['fontSize'] as String? ?? '');
                    if (parsed != null) {
                      final value = (parsed * 100).round();
                      if (value >= minFontSize && value <= maxFontSize) {
                        scope.dispatch({
                          'type': 'toggleStyle',
                          'style': {'type': 'font_size', 'size': value},
                        });
                      }
                    }
                  },
                  builder: (context, form) {
                    return ConfirmModal(
                      title: '폰트 크기',
                      onConfirm: () async {
                        await form.submit();
                      },
                      child: HookFormTextField.collapsed(
                        initialValue: activeValue != null ? (activeValue / 100).toString() : '',
                        name: 'fontSize',
                        placeholder: '${minFontSize ~/ 100}-${maxFontSize ~/ 100}',
                        style: const TextStyle(fontSize: 16),
                        autofocus: true,
                        submitOnEnter: false,
                        keyboardType: const TextInputType.numberWithOptions(decimal: true),
                      ),
                    );
                  },
                ),
              );
            } else {
              scope.dispatch({
                'type': 'toggleStyle',
                'style': {'type': 'font_size', 'size': item['value']},
              });
            }
          },
        );
      },
    );
  }
}
