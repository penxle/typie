import 'dart:io';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/color.dart';
import 'package:typie/screens/editor/toolbar/buttons/icon.dart';
import 'package:typie/screens/editor/toolbar/buttons/label.dart';
import 'package:typie/screens/editor/values.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/vertical_divider.dart';

class TextToolbar extends HookWidget {
  const TextToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final data = useValueListenable(scope.data);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      physics: const AlwaysScrollableScrollPhysics(),
      padding: const Pad(horizontal: 16),
      child: Row(
        spacing: 4,
        children: [
          ColorToolbarButton(
            hex:
                editorValues['textColor']?.firstWhere(
                      (e) =>
                          e['value'] ==
                          (proseMirrorState?.getMarkAttributes('text_style')?['textColor'] as String? ??
                              editorDefaultValues['textColor']),
                    )['hex']
                    as String,
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.textColor;
            },
          ),
          LabelToolbarButton(
            color: AppColors.gray_700,
            text:
                editorValues['fontFamily']?.firstWhereOrNull(
                      (e) =>
                          e['value'] ==
                          (proseMirrorState?.getMarkAttributes('text_style')?['fontFamily'] as String? ??
                              editorDefaultValues['fontFamily']),
                    )?['label']
                    as String? ??
                data?.post.entity.site.fonts
                    .firstWhereOrNull(
                      (e) => e.id == proseMirrorState?.getMarkAttributes('text_style')?['fontFamily'] as String?,
                    )
                    ?.name ??
                '(알 수 없음)',
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.fontFamily;
            },
          ),
          LabelToolbarButton(
            color: AppColors.gray_700,
            text:
                editorValues['fontSize']?.firstWhere(
                      (e) =>
                          e['value'] ==
                          (proseMirrorState?.getMarkAttributes('text_style')?['fontSize'] as num? ??
                              editorDefaultValues['fontSize']),
                    )['label']
                    as String,
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.fontSize;
            },
          ),
          const AppVerticalDivider(height: 20),
          IconToolbarButton(
            icon: LucideLightIcons.bold,
            isActive: proseMirrorState?.isMarkActive('bold') ?? false,
            onTap: () async {
              await scope.command('bold');
            },
          ),
          IconToolbarButton(
            icon: LucideLightIcons.italic,
            isActive: proseMirrorState?.isMarkActive('italic') ?? false,
            onTap: () async {
              await scope.command('italic');
            },
          ),
          IconToolbarButton(
            icon: LucideLightIcons.underline,
            isActive: proseMirrorState?.isMarkActive('underline') ?? false,
            onTap: () async {
              await scope.command('underline');
            },
          ),
          IconToolbarButton(
            icon: LucideLightIcons.strikethrough,
            isActive: proseMirrorState?.isMarkActive('strike') ?? false,
            onTap: () async {
              await scope.command('strike');
            },
          ),
          if (Platform.isIOS) ...[
            const AppVerticalDivider(height: 20),
            IconToolbarButton(
              icon: LucideLightIcons.link,
              isActive: proseMirrorState?.isMarkActive('link') ?? false,
              onTap: () async {
                await context.showModal(
                  intercept: true,
                  child: HookForm(
                    onSubmit: (form) async {
                      await scope.command('link', attrs: {'url': form.data['url']});
                    },
                    builder: (context, form) {
                      return ConfirmModal(
                        title: '링크 삽입',
                        confirmText: '삽입',
                        onConfirm: () async {
                          await form.submit();
                        },
                        child: HookFormTextField.collapsed(
                          initialValue: (proseMirrorState?.getMarkAttributes('link')?['href'] ?? '') as String,
                          name: 'url',
                          placeholder: 'https://...',
                          style: const TextStyle(fontSize: 16),
                          autofocus: true,
                          submitOnEnter: false,
                          keyboardType: TextInputType.url,
                        ),
                      );
                    },
                  ),
                );
              },
            ),
            IconToolbarButton(
              icon: TypieIcons.ruby,
              isActive: proseMirrorState?.isMarkActive('ruby') ?? false,
              onTap: () async {
                await context.showModal(
                  intercept: true,
                  child: HookForm(
                    onSubmit: (form) async {
                      await scope.command('ruby', attrs: {'text': form.data['text']});
                    },
                    builder: (context, form) {
                      return ConfirmModal(
                        title: '루비 삽입',
                        confirmText: '삽입',
                        onConfirm: () async {
                          await form.submit();
                        },
                        child: HookFormTextField.collapsed(
                          initialValue: (proseMirrorState?.getMarkAttributes('ruby')?['text'] ?? '') as String,
                          name: 'text',
                          placeholder: '텍스트 위에 들어갈 문구',
                          style: const TextStyle(fontSize: 16),
                          autofocus: true,
                          submitOnEnter: false,
                          keyboardType: TextInputType.text,
                        ),
                      );
                    },
                  ),
                );
              },
            ),
          ],
          const AppVerticalDivider(height: 20),
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
        ],
      ),
    );
  }
}
