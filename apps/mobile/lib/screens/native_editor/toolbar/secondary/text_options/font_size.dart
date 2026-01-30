import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/label.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';

const minFontSize = 1;
const maxFontSize = 200;

class NativeEditorFontSizeTextOptionsToolbar extends HookWidget {
  const NativeEditorFontSizeTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final uniformMarks = useValueListenable(scope.uniformMarks);
    final mixedMarks = useValueListenable(scope.mixedMarks);

    final isMixed = mixedMarks.contains('font_size');
    final fontSizeMark = uniformMarks.firstWhereOrNull((m) => m['type'] == 'font_size');
    final activeValue = isMixed ? null : (fontSizeMark?['size'] as num? ?? editorDefaultValues['fontSize'] as num);

    final presetItems = editorValues['fontSize']!;
    final allItems = useMemoized(() {
      final items = List<Map<String, dynamic>>.from(presetItems);
      if (activeValue == null) {
        return items;
      }
      final activeNum = activeValue;

      if (!items.any((item) => item['value'] == activeNum)) {
        var insertIndex = items.indexWhere((item) => (item['value'] as num) > activeNum);
        if (insertIndex == -1) {
          insertIndex = items.length;
        }
        final label = activeNum % 1 == 0 ? activeNum.toInt().toString() : activeNum.toString();
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
          onTap: () async {
            if (isActive) {
              await context.showModal(
                intercept: true,
                child: HookForm(
                  onSubmit: (form) async {
                    final value = num.tryParse(form.data['fontSize'] as String? ?? '');
                    if (value != null && value >= minFontSize && value <= maxFontSize) {
                      scope.dispatch({'type': 'setFontSize', 'size': value});
                    }
                  },
                  builder: (context, form) {
                    return ConfirmModal(
                      title: '폰트 크기',
                      onConfirm: () async {
                        await form.submit();
                      },
                      child: HookFormTextField.collapsed(
                        initialValue: activeValue?.toString() ?? '',
                        name: 'fontSize',
                        placeholder: '$minFontSize-$maxFontSize',
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
              scope.dispatch({'type': 'setFontSize', 'size': item['value']});
            }
          },
        );
      },
    );
  }
}
