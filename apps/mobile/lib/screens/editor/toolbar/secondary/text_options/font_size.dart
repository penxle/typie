import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/label.dart';
import 'package:typie/screens/editor/toolbar/secondary/text_options/base.dart';
import 'package:typie/screens/editor/values.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';

const minFontSize = 1;
const maxFontSize = 200;

class FontSizeTextOptionsToolbar extends HookWidget {
  const FontSizeTextOptionsToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    final activeValue =
        proseMirrorState?.getMarkAttributes('text_style')?['fontSize'] as num? ??
        editorDefaultValues['fontSize'] as num;

    final presetItems = editorValues['fontSize']!;
    final allItems = useMemoized(() {
      final items = List<Map<String, dynamic>>.from(presetItems);
      final activeNum = activeValue;

      // NOTE: 현재 값이 목록에 없으면 적절한 위치에 삽입
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

    return TextOptionsToolbar(
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
                      await scope.command('text_style', attrs: {'fontSize': value});
                    }
                  },
                  builder: (context, form) {
                    return ConfirmModal(
                      title: '폰트 크기',
                      onConfirm: () async {
                        await form.submit();
                      },
                      child: HookFormTextField.collapsed(
                        initialValue: activeValue.toString(),
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
              await scope.command('text_style', attrs: {'fontSize': item['value']});
            }
          },
        );
      },
    );
  }
}
