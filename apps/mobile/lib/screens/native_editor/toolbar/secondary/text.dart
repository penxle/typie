import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/modal.dart';
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
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/vertical_divider.dart';

class NativeEditorTextToolbar extends HookWidget {
  const NativeEditorTextToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final attrs = useValueListenable(scope.attrs);
    final selection = useValueListenable(scope.selection);
    final collapsed = selection?.collapsed ?? true;

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

    final boldAttr = findAttr(attrs, 'bold');
    final hasBoldMarker = boldAttr != null && !(boldAttr['values'] as List).contains(null);
    final isBold = hasBoldMarker || (!isFontWeightMixed && activeFontWeight != null && activeFontWeight >= 700);

    final italicAttr = findAttr(attrs, 'italic');
    final isItalic = italicAttr != null && !(italicAttr['values'] as List).contains(null);

    final underlineAttr = findAttr(attrs, 'underline');
    final isUnderline = underlineAttr != null && !(underlineAttr['values'] as List).contains(null);

    final strikethroughAttr = findAttr(attrs, 'strikethrough');
    final isStrikethrough = strikethroughAttr != null && !(strikethroughAttr['values'] as List).contains(null);

    final linkAttr = findAttr(attrs, 'link');
    final linkValues = (linkAttr?['values'] as List?) ?? [];
    final isLinkActive = linkValues.length == 1 && linkValues[0] != null;
    final isLinkMixed = linkValues.length >= 2;
    final existingLinkHref = isLinkActive ? (linkValues[0] as Map<String, dynamic>)['href'] as String? : null;

    final rubyAttr = findAttr(attrs, 'ruby');
    final rubyValues = (rubyAttr?['values'] as List?) ?? [];
    final isRubyActive = rubyValues.length == 1 && rubyValues[0] != null;
    final isRubyMixed = rubyValues.length >= 2;
    final existingRubyText = isRubyActive ? (rubyValues[0] as Map<String, dynamic>)['text'] as String? : null;

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
                ? '(다양한 폰트 패밀리)'
                : scope.controller.fontManager?.fontFamilies
                          .where((f) => f.familyName == activeFontFamily)
                          .firstOrNull
                          ?.displayName ??
                      activeFontFamily ??
                      '(알 수 없음)',
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.fontFamily;
            },
          ),
          LabelToolbarButton(
            color: context.colors.textSubtle,
            text: isFontWeightMixed
                ? '(다양한 폰트 굵기)'
                : editorValues['fontWeight']?.firstWhereOrNull((e) => e['value'] == activeFontWeight)?['label']
                          as String? ??
                      () {
                        final font = scope.controller.fontManager?.fontFamilies
                            .where((f) => f.familyName == activeFontFamily)
                            .firstOrNull
                            ?.fonts
                            .where((f) => f.weight == activeFontWeight)
                            .lastOrNull;
                        if (font?.subfamilyDisplayName != null) {
                          return '${font!.subfamilyDisplayName} ($activeFontWeight)';
                        }
                        return null;
                      }() ??
                      '(알 수 없는 굵기)',
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.fontWeight;
            },
          ),
          LabelToolbarButton(
            color: context.colors.textSubtle,
            text: isFontSizeMixed
                ? '(다양한 폰트 크기)'
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
            icon: LucideLightIcons.link,
            isActive: isLinkActive,
            isDisabled: isLinkMixed || (collapsed && !isLinkActive),
            onTap: () {
              unawaited(
                _showAnnotationModal(
                  context,
                  scope: scope,
                  title: '링크',
                  placeholder: 'https://...',
                  existingValue: existingLinkHref,
                  keyboardType: TextInputType.url,
                  onSubmit: (value) {
                    final url = RegExp(r'^[^:]+:\/\/').hasMatch(value) ? value : 'https://$value';
                    final type = isLinkActive ? 'updateAnnotation' : 'addAnnotation';
                    scope.dispatch({
                      'type': type,
                      'annotation': {'type': 'link', 'href': url},
                    });
                  },
                  onRemove: isLinkActive
                      ? () {
                          scope.dispatch({'type': 'removeAnnotation', 'annotationType': 'link'});
                        }
                      : null,
                ),
              );
            },
          ),
          IconToolbarButton(
            icon: TypieIcons.ruby,
            isActive: isRubyActive,
            isDisabled: isRubyMixed || (collapsed && !isRubyActive),
            onTap: () {
              unawaited(
                _showAnnotationModal(
                  context,
                  scope: scope,
                  title: '루비',
                  placeholder: '텍스트 위에 들어갈 문구',
                  existingValue: existingRubyText,
                  onSubmit: (value) {
                    final type = isRubyActive ? 'updateAnnotation' : 'addAnnotation';
                    scope.dispatch({
                      'type': type,
                      'annotation': {'type': 'ruby', 'text': value},
                    });
                  },
                  onRemove: isRubyActive
                      ? () {
                          scope.dispatch({'type': 'removeAnnotation', 'annotationType': 'ruby'});
                        }
                      : null,
                ),
              );
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

  Future<void> _showAnnotationModal(
    BuildContext context, {
    required NativeEditorToolbarScope scope,
    required String title,
    required String placeholder,
    required String? existingValue,
    required void Function(String value) onSubmit,
    void Function()? onRemove,
    TextInputType? keyboardType,
  }) async {
    await context.showModal(
      intercept: true,
      child: HookForm(
        onSubmit: (form) async {
          final value = form.data['value'] as String?;
          if (value == null || value.isEmpty) {
            return;
          }
          onSubmit(value);
        },
        builder: (context, form) {
          return ConfirmModal(
            title: title,
            confirmText: existingValue != null ? '수정' : '삽입',
            onConfirm: () async {
              await form.submit();
            },
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                HookFormTextField.collapsed(
                  name: 'value',
                  placeholder: placeholder,
                  initialValue: existingValue,
                  style: const TextStyle(fontSize: 16),
                  autofocus: true,
                  submitOnEnter: false,
                  keyboardType: keyboardType,
                ),
                if (onRemove != null) ...[
                  const SizedBox(height: 12),
                  GestureDetector(
                    onTap: () async {
                      onRemove();
                      await context.router.maybePop();
                    },
                    child: Text('제거', style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
                  ),
                ],
              ],
            ),
          );
        },
      ),
    );
  }
}
