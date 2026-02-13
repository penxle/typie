import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/screens/native_editor/state/theme.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/background_color.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/color.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/icon.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/label.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';
import 'package:typie/widgets/vertical_divider.dart';

class NativeEditorTextToolbar extends HookWidget {
  const NativeEditorTextToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final attrs = useValueListenable(scope.attrs);

    final textColorAttr = findAttr(attrs, 'text_color');
    final textColorValues = (textColorAttr?['values'] as List?)?.whereType<String>().toList() ?? [];
    final activeTextColor = textColorValues.length == 1
        ? textColorValues[0]
        : (textColorValues.isEmpty ? editorDefaultValues['textColor'] as String : null);

    final bgColorAttr = findAttr(attrs, 'background_color');
    final bgColorValues = (bgColorAttr?['values'] as List?)?.whereType<String>().toList() ?? [];
    final activeBackgroundColor = bgColorValues.length == 1
        ? bgColorValues[0]
        : (bgColorValues.isEmpty ? editorDefaultValues['textBackgroundColor'] as String : null);

    final fontFamilyAttr = findAttr(attrs, 'font_family');
    final fontFamilyValues = (fontFamilyAttr?['values'] as List?)?.whereType<String>().toList() ?? [];
    final activeFontFamily = fontFamilyValues.length == 1
        ? fontFamilyValues[0]
        : (fontFamilyValues.isEmpty ? editorDefaultValues['fontFamily'] as String : null);
    final isFontFamilyMixed = fontFamilyValues.length > 1;

    final fontWeightAttr = findAttr(attrs, 'font_weight');
    final fontWeightValues = (fontWeightAttr?['values'] as List?)?.whereType<int>().toList() ?? [];
    final activeFontWeight = fontWeightValues.length == 1
        ? fontWeightValues[0]
        : (fontWeightValues.isEmpty ? editorDefaultValues['fontWeight'] as int : null);
    final isFontWeightMixed = fontWeightValues.length > 1;

    final fontSizeAttr = findAttr(attrs, 'font_size');
    final fontSizeValues = (fontSizeAttr?['values'] as List?)?.whereType<num>().toList() ?? [];
    final activeFontSize = fontSizeValues.length == 1
        ? fontSizeValues[0]
        : (fontSizeValues.isEmpty ? editorDefaultValues['fontSize'] as num : null);
    final isFontSizeMixed = fontSizeValues.length > 1;

    final isBold = !isFontWeightMixed && activeFontWeight != null && activeFontWeight >= 700;

    final italicAttr = findAttr(attrs, 'italic');
    final isItalic = italicAttr != null && !(italicAttr['values'] as List).contains(null);

    final underlineAttr = findAttr(attrs, 'underline');
    final isUnderline = underlineAttr != null && !(underlineAttr['values'] as List).contains(null);

    final strikethroughAttr = findAttr(attrs, 'strikethrough');
    final isStrikethrough = strikethroughAttr != null && !(strikethroughAttr['values'] as List).contains(null);

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const Pad(horizontal: 16),
      child: Row(
        spacing: 4,
        children: [
          ColorToolbarButton(
            color:
                (editorValues['textColor']?.firstWhereOrNull((e) => e['value'] == activeTextColor)?['color']
                        as Color Function(BuildContext)?)
                    ?.call(context) ??
                getEditorColor(context.theme.brightness, 'text.black'),
            value: activeTextColor ?? 'black',
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.textColor;
            },
          ),
          BackgroundColorToolbarButton(
            color:
                (editorValues['textBackgroundColor']?.firstWhereOrNull(
                          (e) => e['value'] == activeBackgroundColor,
                        )?['color']
                        as Color Function(BuildContext)?)
                    ?.call(context),
            value: activeBackgroundColor ?? 'none',
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.textBackgroundColor;
            },
          ),
          LabelToolbarButton(
            color: context.colors.textSubtle,
            text: isFontFamilyMixed
                ? '-'
                : editorValues['fontFamily']?.firstWhereOrNull((e) => e['value'] == activeFontFamily)?['label']
                          as String? ??
                      activeFontFamily ??
                      '(알 수 없음)',
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.fontFamily;
            },
          ),
          LabelToolbarButton(
            color: context.colors.textSubtle,
            text: isFontWeightMixed
                ? '-'
                : editorValues['fontWeight']?.firstWhereOrNull((e) => e['value'] == activeFontWeight)?['label']
                          as String? ??
                      activeFontWeight?.toString() ??
                      '(알 수 없음)',
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.fontWeight;
            },
          ),
          LabelToolbarButton(
            color: context.colors.textSubtle,
            text: isFontSizeMixed
                ? '-'
                : () {
                    final size = activeFontSize;
                    if (size == null) {
                      return '-';
                    }
                    return size % 1 == 0 ? size.toInt().toString() : size.toString();
                  }(),
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.fontSize;
            },
          ),
          AppVerticalDivider(color: context.colors.borderSubtle, height: 20),
          IconToolbarButton(
            icon: LucideLightIcons.bold,
            isActive: isBold,
            onTap: () {
              scope.dispatch({'type': 'toggleBold'});
            },
          ),
          IconToolbarButton(
            icon: LucideLightIcons.italic,
            isActive: isItalic,
            onTap: () {
              scope.dispatch({
                'type': 'toggleStyle',
                'style': {'type': 'italic'},
              });
            },
          ),
          IconToolbarButton(
            icon: LucideLightIcons.underline,
            isActive: isUnderline,
            onTap: () {
              scope.dispatch({
                'type': 'toggleStyle',
                'style': {'type': 'underline'},
              });
            },
          ),
          IconToolbarButton(
            icon: LucideLightIcons.strikethrough,
            isActive: isStrikethrough,
            onTap: () {
              scope.dispatch({
                'type': 'toggleStyle',
                'style': {'type': 'strikethrough'},
              });
            },
          ),
          AppVerticalDivider(color: context.colors.borderSubtle, height: 20),
          IconToolbarButton(
            icon: LucideLightIcons.align_left,
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.textAlign;
            },
          ),
          IconToolbarButton(
            icon: TypieIcons.line_height,
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.lineHeight;
            },
          ),
          IconToolbarButton(
            icon: TypieIcons.letter_spacing,
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.letterSpacing;
            },
          ),
          AppVerticalDivider(color: context.colors.borderSubtle, height: 20),
          IconToolbarButton(
            icon: LucideLightIcons.remove_formatting,
            onTap: () {
              scope.dispatch({'type': 'clearFormatting'});
            },
          ),
        ],
      ),
    );
  }
}
