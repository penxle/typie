import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:path_provider/path_provider.dart';
import 'package:share_plus/share_plus.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/loader.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/native_editor/__generated__/export_document_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/sheet/settings.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/select.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/plan_upgrade_bottom_sheet.dart';
import 'package:typie/widgets/popover/popover.dart';
import 'package:typie/widgets/tappable.dart';

class ExportSheet extends HookWidget {
  const ExportSheet({
    required this.documentId,
    required this.client,
    required this.layout,
    required this.hasSubscription,
    super.key,
  });

  final String documentId;
  final GraphQLClient client;
  final Layout? layout;
  final bool hasSubscription;

  @override
  Widget build(BuildContext context) {
    final mixpanel = useService<Mixpanel>();
    final format = useState(GDocumentExportFormat.PDF);
    final useCurrentSettings = useState(true);

    final paginatedLayout = switch (layout) {
      final PaginatedLayout l => l,
      _ => null,
    };
    final isPaginated = paginatedLayout != null;

    final pageWidthMm = useState(
      paginatedLayout != null ? pxToMm(paginatedLayout.pageWidth) : pageSizeMap['a4']!['width']!,
    );
    final pageHeightMm = useState(
      paginatedLayout != null ? pxToMm(paginatedLayout.pageHeight) : pageSizeMap['a4']!['height']!,
    );
    final marginTopMm = useState(
      paginatedLayout != null ? pxToMm(paginatedLayout.pageMarginTop) : defaultPageMargins['a4']!['top']!,
    );
    final marginBottomMm = useState(
      paginatedLayout != null ? pxToMm(paginatedLayout.pageMarginBottom) : defaultPageMargins['a4']!['bottom']!,
    );
    final marginLeftMm = useState(
      paginatedLayout != null ? pxToMm(paginatedLayout.pageMarginLeft) : defaultPageMargins['a4']!['left']!,
    );
    final marginRightMm = useState(
      paginatedLayout != null ? pxToMm(paginatedLayout.pageMarginRight) : defaultPageMargins['a4']!['right']!,
    );

    final isEpub = format.value == GDocumentExportFormat.EPUB;
    final layoutDisabled = isEpub || (isPaginated && useCurrentSettings.value);

    String getPreset() {
      for (final entry in pageSizeMap.entries) {
        if (entry.value['width'] == pageWidthMm.value && entry.value['height'] == pageHeightMm.value) {
          return entry.key;
        }
      }
      return 'custom';
    }

    void handlePresetChange(String preset) {
      if (preset == 'custom') {
        return;
      }
      final size = pageSizeMap[preset]!;
      final margins = defaultPageMargins[preset]!;
      pageWidthMm.value = size['width']!;
      pageHeightMm.value = size['height']!;
      marginTopMm.value = margins['top']!;
      marginBottomMm.value = margins['bottom']!;
      marginLeftMm.value = margins['left']!;
      marginRightMm.value = margins['right']!;
    }

    Future<void> editDimension(String dimension) async {
      final currentValue = dimension == 'width' ? pageWidthMm.value : pageHeightMm.value;
      final label = dimension == 'width' ? '너비' : '높이';
      num? newValue;

      bool validate(HookFormController form, String? valueStr) {
        if (valueStr == null || valueStr.isEmpty) {
          form.setError('value', '값을 입력해주세요');
          return false;
        }
        final value = num.parse(valueStr);
        if (value < minPageSizeMm) {
          form.setError('value', '최소 ${minPageSizeMm.toStringAsFixed(0)} mm 이상이어야 합니다');
          return false;
        }
        if (value > maxPageSizeMm) {
          form.setError('value', '최대 ${maxPageSizeMm.toStringAsFixed(0)} mm 이하여야 합니다');
          return false;
        }
        form.clearError('value');
        return true;
      }

      await context.showModal(
        intercept: true,
        child: HookForm(
          onSubmit: (form) async {
            final valueStr = form.data['value'] as String?;
            if (validate(form, valueStr)) {
              newValue = num.parse(valueStr!);
            }
          },
          builder: (context, form) {
            return ConfirmModal(
              title: '페이지 $label 설정',
              confirmText: '저장',
              onConfirmValidate: () async {
                await form.submit();
                return form.errors['value'] == null && newValue != null;
              },
              onConfirm: () {},
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 8,
                children: [
                  Text('페이지 $label를 입력하세요 (mm)', style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
                  HookFormTextField.collapsed(
                    initialValue: currentValue.toStringAsFixed(0),
                    name: 'value',
                    placeholder: '$label (mm)',
                    autofocus: true,
                    style: const TextStyle(fontSize: 16),
                    submitOnEnter: false,
                    keyboardType: TextInputType.number,
                    inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                    onChanged: (String value) {
                      validate(form, value);
                    },
                  ),
                ],
              ),
            );
          },
        ),
      );

      if (newValue != null && newValue != currentValue) {
        final nextWidthMm = dimension == 'width' ? newValue!.toDouble() : pageWidthMm.value;
        final nextHeightMm = dimension == 'height' ? newValue!.toDouble() : pageHeightMm.value;
        final normalized = normalizeMarginsForPageSize(
          pageWidthMm: nextWidthMm,
          pageHeightMm: nextHeightMm,
          marginTopMm: marginTopMm.value,
          marginBottomMm: marginBottomMm.value,
          marginLeftMm: marginLeftMm.value,
          marginRightMm: marginRightMm.value,
        );
        pageWidthMm.value = nextWidthMm;
        pageHeightMm.value = nextHeightMm;
        marginTopMm.value = normalized['top']!;
        marginBottomMm.value = normalized['bottom']!;
        marginLeftMm.value = normalized['left']!;
        marginRightMm.value = normalized['right']!;
      }
    }

    Future<void> editMargin(String side) async {
      final currentValue = switch (side) {
        'top' => marginTopMm.value,
        'bottom' => marginBottomMm.value,
        'left' => marginLeftMm.value,
        'right' => marginRightMm.value,
        _ => 0.0,
      };
      final label = switch (side) {
        'top' => '위',
        'bottom' => '아래',
        'left' => '왼쪽',
        'right' => '오른쪽',
        _ => '',
      };

      // Compute max margin for this side
      final widthMm = pageWidthMm.value;
      final heightMm = pageHeightMm.value;
      final maxMargin = switch (side) {
        'left' => (widthMm - marginRightMm.value - minContentSizeMm).clamp(0, double.infinity),
        'right' => (widthMm - marginLeftMm.value - minContentSizeMm).clamp(0, double.infinity),
        'top' => (heightMm - marginBottomMm.value - minContentSizeMm).clamp(0, double.infinity),
        'bottom' => (heightMm - marginTopMm.value - minContentSizeMm).clamp(0, double.infinity),
        _ => 0.0,
      };

      num? newValue;

      bool validate(HookFormController form, String? valueStr) {
        if (valueStr == null || valueStr.isEmpty) {
          form.setError('value', '값을 입력해주세요');
          return false;
        }
        final value = num.parse(valueStr);
        if (value > maxMargin) {
          form.setError('value', '최대 ${maxMargin.toStringAsFixed(0)}mm까지 가능합니다');
          return false;
        }
        form.clearError('value');
        return true;
      }

      await context.showModal(
        intercept: true,
        child: HookForm(
          onSubmit: (form) async {
            final valueStr = form.data['value'] as String?;
            if (validate(form, valueStr)) {
              newValue = num.parse(valueStr!);
            }
          },
          builder: (context, form) {
            return ConfirmModal(
              title: '$label 여백 설정',
              confirmText: '저장',
              onConfirmValidate: () async {
                await form.submit();
                return form.errors['value'] == null && newValue != null;
              },
              onConfirm: () {},
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 8,
                children: [
                  Text(
                    '$label 여백을 입력하세요 (0-${maxMargin.toStringAsFixed(0)}mm)',
                    style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                  ),
                  HookFormTextField.collapsed(
                    initialValue: currentValue.toStringAsFixed(0),
                    name: 'value',
                    placeholder: '여백 (mm)',
                    autofocus: true,
                    style: const TextStyle(fontSize: 16),
                    submitOnEnter: false,
                    keyboardType: TextInputType.number,
                    inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                    onChanged: (String value) {
                      validate(form, value);
                    },
                  ),
                ],
              ),
            );
          },
        ),
      );

      if (newValue != null && newValue != currentValue) {
        switch (side) {
          case 'top':
            marginTopMm.value = newValue!.toDouble();
          case 'bottom':
            marginBottomMm.value = newValue!.toDouble();
          case 'left':
            marginLeftMm.value = newValue!.toDouble();
          case 'right':
            marginRightMm.value = newValue!.toDouble();
        }
      }
    }

    Future<void> handleExport() async {
      try {
        await context.runWithLoader(() async {
          final Value<GExportDocumentPageLayoutInput> layoutInput;
          if (isEpub) {
            layoutInput = const Value.absent();
          } else if (isPaginated && useCurrentSettings.value) {
            layoutInput = Value.present(
              GExportDocumentPageLayoutInput(
                (b) => b
                  ..pageWidth = mmToPx(pxToMm(paginatedLayout.pageWidth)).toInt()
                  ..pageHeight = mmToPx(pxToMm(paginatedLayout.pageHeight)).toInt()
                  ..pageMarginTop = mmToPx(pxToMm(paginatedLayout.pageMarginTop)).toInt()
                  ..pageMarginBottom = mmToPx(pxToMm(paginatedLayout.pageMarginBottom)).toInt()
                  ..pageMarginLeft = mmToPx(pxToMm(paginatedLayout.pageMarginLeft)).toInt()
                  ..pageMarginRight = mmToPx(pxToMm(paginatedLayout.pageMarginRight)).toInt(),
              ),
            );
          } else {
            layoutInput = Value.present(
              GExportDocumentPageLayoutInput(
                (b) => b
                  ..pageWidth = mmToPx(pageWidthMm.value).toInt()
                  ..pageHeight = mmToPx(pageHeightMm.value).toInt()
                  ..pageMarginTop = mmToPx(marginTopMm.value).toInt()
                  ..pageMarginBottom = mmToPx(marginBottomMm.value).toInt()
                  ..pageMarginLeft = mmToPx(marginLeftMm.value).toInt()
                  ..pageMarginRight = mmToPx(marginRightMm.value).toInt(),
              ),
            );
          }

          final res = await client.request(
            GExportSheet_ExportDocument_MutationReq(
              (b) => b.vars.input
                ..documentId = documentId
                ..format = format.value
                ..layout = layoutInput,
            ),
          );

          final result = res.exportDocument;
          final bytes = base64Decode(result.data.value);
          final dir = await getTemporaryDirectory();
          final file = File('${dir.path}/${result.filename}');
          await file.writeAsBytes(bytes);

          await SharePlus.instance.share(ShareParams(files: [XFile(file.path, mimeType: result.mimeType)]));

          unawaited(mixpanel.track('export_document', properties: {'format': format.value.name}));
        });

        if (context.mounted) {
          Navigator.of(context).pop();
        }
      } catch (_) {
        if (context.mounted) {
          context.toast(ToastType.error, '내보내기에 실패했어요. 잠시 후 다시 시도해주세요.');
        }
      }
    }

    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _FormatSection(format: format, hasSubscription: hasSubscription),
          if (format.value == GDocumentExportFormat.HWP || format.value == GDocumentExportFormat.DOCX) ...[
            const Gap(8),
            Text(
              '파일 특성상 일부 서식과 페이지 분할이 다르게 표시될 수 있어요.',
              style: TextStyle(fontSize: 13, color: context.colors.textFaint),
            ),
          ],
          if (isEpub) ...[
            const Gap(8),
            Text(
              '전자책 특성상 문서에 포함된 장식 요소들이 간소화되고, 페이지 레이아웃이 적용되지 않아요.',
              style: TextStyle(fontSize: 13, color: context.colors.textFaint),
            ),
          ],
          const Gap(16),
          HorizontalDivider(color: context.colors.borderDefault),
          const Gap(16),
          if (isPaginated) ...[
            Tappable(
              onTap: () => useCurrentSettings.value = !useCurrentSettings.value,
              child: Row(
                spacing: 8,
                children: [
                  SizedBox(
                    width: 20,
                    height: 20,
                    child: Checkbox(
                      value: useCurrentSettings.value,
                      onChanged: (v) => useCurrentSettings.value = v ?? false,
                      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                      visualDensity: VisualDensity.compact,
                    ),
                  ),
                  Text('현재 페이지 설정 사용', style: TextStyle(fontSize: 14, color: context.colors.textDefault)),
                ],
              ),
            ),
            const Gap(16),
          ],
          _PageSizeSection(
            disabled: layoutDisabled,
            preset: getPreset(),
            widthMm: pageWidthMm.value,
            heightMm: pageHeightMm.value,
            onPresetChange: handlePresetChange,
            onEditDimension: editDimension,
          ),
          const Gap(16),
          _PageMarginSection(
            disabled: layoutDisabled,
            topMm: marginTopMm.value,
            bottomMm: marginBottomMm.value,
            leftMm: marginLeftMm.value,
            rightMm: marginRightMm.value,
            onEditMargin: editMargin,
          ),
          const Gap(24),
          Tappable(
            onTap: handleExport,
            child: Container(
              decoration: BoxDecoration(color: context.colors.surfaceInverse, borderRadius: BorderRadius.circular(8)),
              padding: const Pad(vertical: 16),
              child: Text(
                '내보내기',
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: context.colors.textInverse),
                textAlign: TextAlign.center,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _FormatSection extends StatelessWidget {
  const _FormatSection({required this.format, required this.hasSubscription});

  final ValueNotifier<GDocumentExportFormat> format;
  final bool hasSubscription;

  @override
  Widget build(BuildContext context) {
    return HookForm(
      submitMode: HookFormSubmitMode.onChange,
      onSubmit: (form) async {
        final dirtyData = form.getDirtyFieldsData();
        if (dirtyData.containsKey('format')) {
          final selected = dirtyData['format'] as GDocumentExportFormat;
          if (!hasSubscription && selected != GDocumentExportFormat.PDF) {
            form.setValue('format', GDocumentExportFormat.PDF);
            format.value = GDocumentExportFormat.PDF;

            final result = await context.showBottomSheet<PlanUpgradeResult>(
              intercept: true,
              child: const PlanUpgradeBottomSheet(message: '파일 내보내기는 FULL ACCESS 플랜에서 사용할 수 있어요.'),
            );

            if (result == PlanUpgradeResult.upgrade && context.mounted) {
              context.router.popUntilRoot();
              await context.router.popAndPush(const EnrollPlanRoute());
            }

            return;
          }

          format.value = selected;
        }
      },
      builder: (context, form) {
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          spacing: 8,
          children: [
            Text('파일 형식', style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
            HookFormSelect(
              name: 'format',
              initialValue: format.value,
              position: PopoverPosition.bottomLeft,
              items: const [
                HookFormSelectItem(
                  icon: TypieIcons.file_pdf,
                  label: 'PDF (Acrobat)',
                  description: '인쇄와 공유에 적합한 고정 레이아웃',
                  value: GDocumentExportFormat.PDF,
                ),
                HookFormSelectItem(
                  icon: TypieIcons.file_hwp,
                  label: 'HWP (한/글)',
                  description: '편집 가능한 한컴오피스 호환 문서',
                  value: GDocumentExportFormat.HWP,
                ),
                HookFormSelectItem(
                  icon: TypieIcons.file_docx,
                  label: 'DOCX (워드)',
                  description: '편집 가능한 Microsoft Word 호환 문서',
                  value: GDocumentExportFormat.DOCX,
                ),
                HookFormSelectItem(
                  icon: TypieIcons.file_epub,
                  label: 'EPUB (전자책)',
                  description: '전자책 리더에서 읽을 수 있는 표준 문서',
                  value: GDocumentExportFormat.EPUB,
                ),
              ],
            ),
          ],
        );
      },
    );
  }
}

class _PageSizeSection extends StatelessWidget {
  const _PageSizeSection({
    required this.disabled,
    required this.preset,
    required this.widthMm,
    required this.heightMm,
    required this.onPresetChange,
    required this.onEditDimension,
  });

  final bool disabled;
  final String preset;
  final double widthMm;
  final double heightMm;
  final void Function(String) onPresetChange;
  final Future<void> Function(String) onEditDimension;

  @override
  Widget build(BuildContext context) {
    final subtleColor = disabled ? context.colors.textFaint : context.colors.textSubtle;

    return IgnorePointer(
      ignoring: disabled,
      child: Opacity(
        opacity: disabled ? 0.4 : 1.0,
        child: HookForm(
          submitMode: HookFormSubmitMode.onChange,
          onSubmit: (form) async {
            final dirtyData = form.getDirtyFieldsData();
            if (dirtyData.containsKey('pageSize')) {
              onPresetChange(dirtyData['pageSize'] as String);
            }
          },
          builder: (context, form) {
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 8,
              children: [
                Row(
                  children: [
                    Icon(LucideLightIcons.file, size: 20, color: subtleColor),
                    const Gap(8),
                    Expanded(
                      child: Text('페이지 크기 (mm)', style: TextStyle(fontSize: 14, color: subtleColor)),
                    ),
                    HookFormSelect(
                      name: 'pageSize',
                      initialValue: preset,
                      items: const [
                        HookFormSelectItem(label: 'A4 (210×297)', value: 'a4'),
                        HookFormSelectItem(label: 'A5 (148×210)', value: 'a5'),
                        HookFormSelectItem(label: 'B5 (176×250)', value: 'b5'),
                        HookFormSelectItem(label: 'B6 (125×176)', value: 'b6'),
                        HookFormSelectItem(label: '직접 지정', value: 'custom'),
                      ],
                    ),
                  ],
                ),
                Row(
                  spacing: 8,
                  children: [
                    Tappable(
                      onTap: () => onEditDimension('width'),
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                        decoration: BoxDecoration(
                          border: Border.all(color: context.colors.borderStrong, width: 0.5),
                          borderRadius: BorderRadius.circular(6),
                        ),
                        child: Text(
                          '너비: ${widthMm.toStringAsFixed(0)}mm',
                          style: TextStyle(fontSize: 14, color: subtleColor),
                        ),
                      ),
                    ),
                    Tappable(
                      onTap: () => onEditDimension('height'),
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                        decoration: BoxDecoration(
                          border: Border.all(color: context.colors.borderStrong, width: 0.5),
                          borderRadius: BorderRadius.circular(6),
                        ),
                        child: Text(
                          '높이: ${heightMm.toStringAsFixed(0)}mm',
                          style: TextStyle(fontSize: 14, color: subtleColor),
                        ),
                      ),
                    ),
                  ],
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _PageMarginSection extends StatelessWidget {
  const _PageMarginSection({
    required this.disabled,
    required this.topMm,
    required this.bottomMm,
    required this.leftMm,
    required this.rightMm,
    required this.onEditMargin,
  });

  final bool disabled;
  final double topMm;
  final double bottomMm;
  final double leftMm;
  final double rightMm;
  final Future<void> Function(String) onEditMargin;

  Widget _marginButton(BuildContext context, String side, String label, double valueMm) {
    return Tappable(
      onTap: () => onEditMargin(side),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(
          border: Border.all(color: context.colors.borderStrong, width: 0.5),
          borderRadius: BorderRadius.circular(6),
        ),
        child: Text(
          '$label: ${valueMm.toStringAsFixed(0)}',
          style: TextStyle(fontSize: 14, color: disabled ? context.colors.textFaint : context.colors.textSubtle),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final subtleColor = disabled ? context.colors.textFaint : context.colors.textSubtle;

    return IgnorePointer(
      ignoring: disabled,
      child: Opacity(
        opacity: disabled ? 0.4 : 1.0,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          spacing: 8,
          children: [
            Row(
              children: [
                Icon(LucideLightIcons.ruler_dimension_line, size: 20, color: subtleColor),
                const Gap(8),
                Text('여백 (mm)', style: TextStyle(fontSize: 14, color: subtleColor)),
              ],
            ),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                _marginButton(context, 'top', '위', topMm),
                _marginButton(context, 'bottom', '아래', bottomMm),
                _marginButton(context, 'left', '왼쪽', leftMm),
                _marginButton(context, 'right', '오른쪽', rightMm),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
