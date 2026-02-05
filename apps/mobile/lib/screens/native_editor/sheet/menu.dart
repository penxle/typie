import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/native_editor/__generated__/delete_document_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/duplicate_document_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/screens/native_editor/sheet/info.dart';
import 'package:typie/screens/native_editor/sheet/settings.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:url_launcher/url_launcher.dart';

class MenuSheet extends StatelessWidget {
  const MenuSheet({
    required this.data,
    required this.document,
    required this.client,
    required this.editor,
    required this.editorController,
    required this.onOpenFindReplace,
    required this.onOpenSpellcheck,
    super.key,
  });

  final GNativeEditorScreen_QueryData data;
  final GNativeEditorScreen_QueryData_entity_node__asDocument document;
  final GraphQLClient client;
  final NativeEditor? editor;
  final EditorController? editorController;
  final VoidCallback onOpenFindReplace;
  final VoidCallback onOpenSpellcheck;

  @override
  Widget build(BuildContext context) {
    return AppBottomSheet(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          BottomMenuItem(icon: LucideLightIcons.search, label: '찾기', onTap: onOpenFindReplace),
          BottomMenuItem(icon: LucideLightIcons.bookmark, label: '북마크', onTap: () {}),
          BottomMenuItem(icon: LucideLightIcons.spell_check, label: '맞춤법 검사', onTap: onOpenSpellcheck),
          BottomMenuItem(icon: LucideLightIcons.lightbulb, label: 'AI 피드백', onTap: () {}),
          const Gap(16),
          HorizontalDivider(color: context.colors.borderDefault),
          const Gap(16),
          BottomMenuItem(
            icon: LucideLightIcons.info,
            label: '정보',
            onTap: () async {
              final characterCounts = editor?.getCharacterCounts();
              await context.showBottomSheet(
                intercept: true,
                child: InfoSheet(slug: data.entity.slug, client: client, characterCounts: characterCounts),
              );
            },
          ),
          BottomMenuItem(
            icon: LucideLightIcons.settings,
            label: '본문 설정',
            onTap: () async {
              final controller = editorController;
              if (controller == null) {
                return;
              }
              await context.showBottomSheet(intercept: true, child: SettingsSheet(controller: controller));
            },
          ),
          BottomMenuItem(
            icon: LucideLightIcons.external_link,
            label: '스페이스에서 열기',
            onTap: () async {
              final url = Uri.parse(data.entity.url);
              await launchUrl(url, mode: LaunchMode.externalApplication);
            },
          ),
          BottomMenuItem(icon: LucideLightIcons.blend, label: '공유하기', onTap: () {}),
          BottomMenuItem(
            icon: LucideLightIcons.copy,
            label: '복제하기',
            onTap: () async {
              final res = await client.request(
                GNativeEditor_DuplicateDocument_MutationReq((b) => b..vars.input.documentId = document.id),
              );
              if (context.mounted) {
                await context.router.popAndPush(NativeEditorRoute(slug: res.duplicateDocument.entity.slug));
              }
            },
          ),
          BottomMenuItem(
            icon: LucideLightIcons.trash_2,
            label: '삭제하기',
            onTap: () async {
              await context.showModal(
                intercept: true,
                child: ConfirmModal(
                  title: '문서 삭제',
                  message: '"${document.title}" 문서를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.',
                  confirmText: '삭제하기',
                  confirmTextColor: context.colors.textBright,
                  confirmBackgroundColor: context.colors.accentDanger,
                  onConfirm: () async {
                    await client.request(
                      GNativeEditor_DeleteDocument_MutationReq((b) => b..vars.input.documentId = document.id),
                    );
                    if (context.mounted) {
                      await context.router.maybePop();
                    }
                  },
                ),
              );
            },
          ),
        ],
      ),
    );
  }
}
