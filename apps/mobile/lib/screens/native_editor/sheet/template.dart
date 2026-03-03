import 'dart:async';
import 'dart:convert';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/__generated__/load_template_document.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/widgets/tappable.dart';

class TemplateSheet extends HookWidget {
  const TemplateSheet({required this.templates, required this.editor, required this.client, super.key});

  final List<GNativeEditorScreen_QueryData_entity_site_documentTemplates> templates;
  final NativeEditor editor;
  final GraphQLClient client;

  @override
  Widget build(BuildContext context) {
    final isLoading = useState(false);
    final bottomPadding = MediaQuery.paddingOf(context).bottom;

    Future<void> loadTemplate(GNativeEditorScreen_QueryData_entity_site_documentTemplates template) async {
      isLoading.value = true;
      try {
        final resp = await client.request(
          GNativeEditor_LoadTemplateDocument_QueryReq((b) => b..vars.slug = template.entity.slug),
        );

        final snapshotBase64 = resp.document.snapshot.value;
        if (snapshotBase64.isEmpty) {
          if (context.mounted) {
            context.toast(ToastType.error, '템플릿을 불러올 수 없습니다');
          }
          return;
        }

        final snapshot = base64Decode(snapshotBase64);
        editor.insertTemplateFragment(Uint8List.fromList(snapshot));

        if (context.mounted) {
          await context.router.maybePop();
        }
      } catch (_) {
        if (context.mounted) {
          context.toast(ToastType.error, '템플릿을 불러오는 데 실패했습니다');
        }
      } finally {
        if (context.mounted) {
          isLoading.value = false;
        }
      }
    }

    return AppBottomSheet(
      includeBottomPadding: false,
      padding: const Pad(horizontal: 20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            '템플릿 불러오기',
            style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700, color: context.colors.textDefault),
          ),
          const Gap(12),
          if (templates.isEmpty) ...[
            Padding(
              padding: Pad(vertical: 20, bottom: bottomPadding + 12),
              child: Center(
                child: Text(
                  '아직 템플릿이 없어요.\n에디터 상단 더보기 메뉴에서\n기존 문서를 템플릿으로 전환해보세요.',
                  style: TextStyle(fontSize: 14, color: context.colors.textFaint),
                  textAlign: TextAlign.center,
                ),
              ),
            ),
          ] else ...[
            AbsorbPointer(
              absorbing: isLoading.value,
              child: ConstrainedBox(
                constraints: BoxConstraints(maxHeight: MediaQuery.sizeOf(context).height * 0.4),
                child: SingleChildScrollView(
                  padding: Pad(bottom: bottomPadding + 12),
                  child: Column(
                    children: templates
                        .map(
                          (template) => Tappable(
                            onTap: () {
                              unawaited(loadTemplate(template));
                            },
                            child: Container(
                              decoration: BoxDecoration(
                                border: Border.all(color: context.colors.borderStrong),
                                borderRadius: BorderRadius.circular(8),
                              ),
                              padding: const Pad(all: 12),
                              margin: const Pad(bottom: 8),
                              child: Row(
                                children: [
                                  Icon(LucideLightIcons.layout_template, size: 18, color: context.colors.textSubtle),
                                  const Gap(10),
                                  Expanded(
                                    child: Text(
                                      template.title,
                                      style: TextStyle(fontSize: 15, color: context.colors.textDefault),
                                      overflow: TextOverflow.ellipsis,
                                      maxLines: 1,
                                    ),
                                  ),
                                  const Gap(8),
                                  Text('사용하기', style: TextStyle(fontSize: 13, color: context.colors.textFaint)),
                                ],
                              ),
                            ),
                          ),
                        )
                        .toList(),
                  ),
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }
}
