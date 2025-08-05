import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/background_color.dart';
import 'package:typie/screens/editor/toolbar/buttons/color.dart';
import 'package:typie/screens/editor/toolbar/buttons/icon.dart';
import 'package:typie/screens/editor/toolbar/buttons/label.dart';
import 'package:typie/screens/editor/values.dart';
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
            color:
                (editorValues['textColor']?.firstWhere(
                      (e) =>
                          e['value'] ==
                          (proseMirrorState?.getMarkAttributes('text_style')?['textColor'] as String? ??
                              editorDefaultValues['textColor']),
                    )['color']
                    as Color Function(BuildContext))(context),
            value:
                proseMirrorState?.getMarkAttributes('text_style')?['textColor'] as String? ??
                editorDefaultValues['textColor'] as String,
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.textColor;
            },
          ),
          BackgroundColorToolbarButton(
            color:
                (editorValues['textBackgroundColor']?.firstWhere(
                          (e) =>
                              e['value'] ==
                              (proseMirrorState?.getMarkAttributes('text_style')?['textBackgroundColor'] as String? ??
                                  editorDefaultValues['textBackgroundColor']),
                        )['color']
                        as Color Function(BuildContext)?)
                    ?.call(context),
            value:
                proseMirrorState?.getMarkAttributes('text_style')?['textBackgroundColor'] as String? ??
                editorDefaultValues['textBackgroundColor'] as String,
            onTap: () {
              scope.secondaryToolbarMode.value = SecondaryToolbarMode.textBackgroundColor;
            },
          ),
          LabelToolbarButton(
            color: context.colors.textSubtle,
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
            color: context.colors.textSubtle,
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
          AppVerticalDivider(color: context.colors.borderSubtle, height: 20),
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
          AppVerticalDivider(color: context.colors.borderSubtle, height: 20),
          IconToolbarButton(
            icon: LucideLightIcons.link,
            isActive: proseMirrorState?.isMarkActive('link') ?? false,
            onTap: () async {
              // NOTE: Android에서 모달 열리면 범위 selection이 취소되므로 저장해서 씀
              final selection = proseMirrorState?.selection;
              final initialValue = (proseMirrorState?.getMarkAttributes('link')?['href'] ?? '') as String;

              await context.showModal(
                intercept: true,
                child: HookForm(
                  onSubmit: (form) async {
                    await scope.command('link', attrs: {'url': form.data['url'], 'selection': selection});
                  },
                  builder: (context, form) {
                    return ConfirmModal(
                      title: initialValue.isEmpty ? '링크 삽입' : '링크 수정',
                      confirmText: initialValue.isEmpty ? '삽입' : '수정',
                      onConfirm: () async {
                        await form.submit();
                      },
                      child: HookFormTextField.collapsed(
                        initialValue: initialValue,
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
              // NOTE: Android에서 모달 열리면 범위 selection이 취소되므로 저장해서 씀
              final selection = proseMirrorState?.selection;
              final initialValue = (proseMirrorState?.getMarkAttributes('ruby')?['text'] ?? '') as String;

              await context.showModal(
                intercept: true,
                child: HookForm(
                  onSubmit: (form) async {
                    await scope.command('ruby', attrs: {'text': form.data['text'], 'selection': selection});
                  },
                  builder: (context, form) {
                    return ConfirmModal(
                      title: initialValue.isEmpty ? '루비 삽입' : '루비 수정',
                      confirmText: initialValue.isEmpty ? '삽입' : '수정',
                      onConfirm: () async {
                        await form.submit();
                      },
                      child: HookFormTextField.collapsed(
                        initialValue: initialValue,
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
        ],
      ),
    );
  }
}
