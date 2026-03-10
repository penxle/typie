import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/modals/share.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/native_editor/__generated__/delete_document_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/duplicate_document_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/screens/native_editor/__generated__/update_document_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/update_document_type_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/sheet/export.dart';
import 'package:typie/screens/native_editor/sheet/info.dart';
import 'package:typie/screens/native_editor/sheet/settings.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
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
    required this.onOpenRemark,
    required this.onOpenAiFeedback,
    required this.onOpenRelatedNotes,
    this.onSendInputLog,
    super.key,
  });

  final GNativeEditorScreen_QueryData data;
  final GNativeEditorScreen_QueryData_entity_node__asDocument document;
  final GraphQLClient client;
  final NativeEditor? editor;
  final EditorController? editorController;
  final VoidCallback onOpenFindReplace;
  final VoidCallback onOpenSpellcheck;
  final VoidCallback onOpenRemark;
  final VoidCallback onOpenAiFeedback;
  final VoidCallback onOpenRelatedNotes;
  final VoidCallback? onSendInputLog;

  @override
  Widget build(BuildContext context) {
    return AppBottomSheet(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          BottomMenuItem(icon: LucideLightIcons.search, label: '찾기', onTap: onOpenFindReplace),
          BottomMenuItem(icon: LucideLightIcons.sticky_note, label: '노트', onTap: onOpenRelatedNotes),
          BottomMenuItem(
            icon: LucideLightIcons.message_square_text,
            label: '코멘트',
            trailing: (editorController?.state.remarks.length ?? 0) > 0
                ? Container(
                    decoration: BoxDecoration(
                      border: Border.all(color: context.colors.borderStrong),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    padding: const Pad(horizontal: 8, vertical: 4),
                    child: Text(
                      '${editorController?.state.remarks.length ?? 0}',
                      style: TextStyle(fontSize: 12, fontWeight: FontWeight.w500, color: context.colors.textDefault),
                    ),
                  )
                : null,
            onTap: onOpenRemark,
          ),
          BottomMenuItem(icon: LucideLightIcons.spell_check, label: '맞춤법 검사', onTap: onOpenSpellcheck),
          BottomMenuItem(icon: LucideLightIcons.lightbulb, label: 'AI 피드백', onTap: onOpenAiFeedback),
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
            icon: document.locked ? LucideLightIcons.lock_open : LucideLightIcons.lock,
            label: document.locked ? '편집 잠금 해제' : '편집 잠금',
            onTap: () {
              unawaited(
                client.request(
                  GNativeEditor_UpdateDocument_MutationReq(
                    (b) => b.vars.input
                      ..documentId = document.id
                      ..locked = Value.present(!document.locked),
                  ),
                ),
              );
              if (context.mounted) {
                context.toast(ToastType.success, document.locked ? '편집 잠금이 해제되었어요.' : '편집 잠금이 설정되었어요.');
              }
            },
          ),
          BottomMenuItem(
            icon: LucideLightIcons.file_down,
            label: '파일로 내보내기',
            onTap: () async {
              await context.showBottomSheet(
                intercept: true,
                child: ExportSheet(
                  documentId: document.id,
                  client: client,
                  layout: editorController?.state.layout,
                  hasSubscription: data.me?.subscription != null,
                ),
              );
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
          BottomMenuItem(
            icon: LucideLightIcons.blend,
            label: '공유하기',
            trailing:
                data.entity.visibility == GEntityVisibility.PUBLIC ||
                    data.entity.visibility == GEntityVisibility.UNLISTED
                ? Container(
                    decoration: BoxDecoration(
                      border: Border.all(color: context.colors.borderStrong),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    padding: const Pad(horizontal: 8, vertical: 4),
                    child: Text(
                      data.entity.visibility == GEntityVisibility.PUBLIC ? '공개 중' : '링크 공개 중',
                      style: TextStyle(fontSize: 12, fontWeight: FontWeight.w500, color: context.colors.textDefault),
                    ),
                  )
                : null,
            onTap: () async {
              await context.showBottomSheet(intercept: true, child: ShareBottomSheet(entityIds: [data.entity.id]));
            },
          ),
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
            icon: LucideLightIcons.layout_template,
            label: document.type == GDocumentType.TEMPLATE ? '문서로 전환' : '템플릿으로 전환',
            onTap: () async {
              final isToTemplate = document.type != GDocumentType.TEMPLATE;
              await context.showModal(
                intercept: true,
                child: ConfirmModal(
                  title: isToTemplate ? '템플릿으로 전환' : '문서로 전환',
                  message: isToTemplate
                      ? '이 문서를 템플릿으로 전환하시겠어요?\n앞으로 새 문서를 생성할 때 이 문서의 내용을 쉽게 이용할 수 있어요.'
                      : '이 템플릿을 다시 일반 문서로 전환하시겠어요?',
                  confirmText: '전환',
                  onConfirm: () async {
                    await client.request(
                      GNativeEditor_UpdateDocumentType_MutationReq(
                        (b) => b
                          ..vars.input.documentId = document.id
                          ..vars.input.type = isToTemplate ? GDocumentType.TEMPLATE : GDocumentType.NORMAL,
                      ),
                    );
                    if (context.mounted) {
                      await context.router.maybePop();
                    }
                  },
                ),
              );
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
          if (onSendInputLog != null) ...[
            const Gap(16),
            HorizontalDivider(color: context.colors.borderDefault),
            const Gap(16),
            BottomMenuItem(icon: LucideLightIcons.send, label: '입력 로그 보내기', onTap: onSendInputLog!),
          ],
        ],
      ),
    );
  }
}
