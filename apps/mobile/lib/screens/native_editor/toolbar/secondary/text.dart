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
    final uniformStyles = useValueListenable(scope.uniformStyles);
    final mixedStyles = useValueListenable(scope.mixedStyles);

    Map<String, dynamic>? findStyle(String type) => uniformStyles.firstWhereOrNull((m) => m['type'] == type);
    bool isMixed(String type) => mixedStyles.contains(type);

    final textColorStyle = findStyle('text_color');
    final isTextColorMixed = isMixed('text_color');
    final activeTextColor = isTextColorMixed
        ? null
        : (textColorStyle?['color'] as String? ?? editorDefaultValues['textColor'] as String);

    final backgroundColorStyle = findStyle('background_color');
    final isBackgroundColorMixed = isMixed('background_color');
    final activeBackgroundColor = isBackgroundColorMixed
        ? null
        : (backgroundColorStyle?['color'] as String? ?? editorDefaultValues['textBackgroundColor'] as String);

    final fontFamilyStyle = findStyle('font_family');
    final isFontFamilyMixed = isMixed('font_family');
    final activeFontFamily = isFontFamilyMixed
        ? null
        : (fontFamilyStyle?['family'] as String? ?? editorDefaultValues['fontFamily'] as String);

    final fontWeightStyle = findStyle('font_weight');
    final isFontWeightMixed = isMixed('font_weight');
    final activeFontWeight = isFontWeightMixed
        ? null
        : (fontWeightStyle?['weight'] as int? ?? editorDefaultValues['fontWeight'] as int);

    final fontSizeStyle = findStyle('font_size');
    final isFontSizeMixed = isMixed('font_size');
    final activeFontSize = isFontSizeMixed
        ? null
        : (fontSizeStyle?['size'] as num? ?? editorDefaultValues['fontSize'] as num);

    final isBold = !isFontWeightMixed && activeFontWeight != null && activeFontWeight >= 700;

    final isItalicMixed = isMixed('italic');
    final isItalic = !isItalicMixed && uniformStyles.any((m) => m['type'] == 'italic');

    final isUnderlineMixed = isMixed('underline');
    final isUnderline = !isUnderlineMixed && uniformStyles.any((m) => m['type'] == 'underline');

    final isStrikethroughMixed = isMixed('strikethrough');
    final isStrikethrough = !isStrikethroughMixed && uniformStyles.any((m) => m['type'] == 'strikethrough');

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
