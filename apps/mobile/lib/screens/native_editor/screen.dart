import 'dart:async';
import 'dart:convert';
import 'dart:ui' as ui;

import 'package:auto_route/auto_route.dart';
import 'package:ferry/ferry.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/native_editor/__generated__/delete_document_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/duplicate_document_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/update_document_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/body_setting_bottom_sheet.dart';
import 'package:typie/screens/native_editor/fonts.dart';
import 'package:typie/screens/native_editor/state/editor_state.dart';
import 'package:typie/screens/native_editor/sync/document_sync_manager.dart';
import 'package:typie/screens/native_editor/theme.dart';
import 'package:typie/screens/native_editor/util/initializer.dart';
import 'package:typie/screens/native_editor/view/editor_view.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/screen.dart';
import 'package:url_launcher/url_launcher.dart';

@RoutePage()
class NativeEditorScreen extends StatelessWidget {
  const NativeEditorScreen({required this.slug, super.key});

  final String slug;

  @override
  Widget build(BuildContext context) {
    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceDefault,
      operation: GNativeEditorScreen_QueryReq((b) => b.vars.slug = slug),
      builder: (context, client, data) => _Content(data: data, client: client),
    );
  }
}

class _Content extends HookWidget {
  const _Content({required this.data, required this.client});

  final GNativeEditorScreen_QueryData data;
  final GraphQLClient client;

  @override
  Widget build(BuildContext context) {
    final error = useState<String?>(null);
    final app = useRef<NativeEditorApplication?>(null);
    final fontManager = useRef<EditorFontManager?>(null);
    final editor = useState<NativeEditor?>(null);
    final editorController = useRef<EditorController?>(null);
    final editorControllerReady = useState(false);
    final syncManager = useRef<DocumentSyncManager?>(null);

    final localTitle = useState<String>('');
    final localSubtitle = useState<String>('');
    final titleDirty = useState<bool>(false);
    final subtitleDirty = useState<bool>(false);
    final titleFocusNode = useFocusNode();
    final subtitleFocusNode = useFocusNode();
    final titleDebounceTimer = useRef<Timer?>(null);
    final subtitleDebounceTimer = useRef<Timer?>(null);

    final document = data.entity.node.when(document: (doc) => doc, orElse: () => null);
    final headingTitle = document?.title ?? '(제목 없음)';
    final scaleFactor = ui.PlatformDispatcher.instance.views.first.devicePixelRatio;

    final brightness = MediaQuery.platformBrightnessOf(context);

    useEffect(() {
      if (document == null) {
        error.value = 'Document not found';
        return null;
      }

      final theme = getEditorTheme(brightness);

      Future<void> init() async {
        try {
          final snapshotBase64 = document.snapshot.value;
          final snapshot = snapshotBase64.isNotEmpty ? base64Decode(snapshotBase64) : null;

          final (application, manager) = await getOrInitializeApplication();
          app.value = application;
          fontManager.value = manager;
          editor.value = application.createEditor(scaleFactor, snapshot: snapshot)
            ..dispatch({
              'type': 'initialize',
              'theme': {'colors': theme},
            });
        } on EditorException catch (err) {
          error.value = err.message;
        } catch (err) {
          error.value = err.toString();
        }
      }

      unawaited(init());

      return () {
        if (titleDebounceTimer.value != null) {
          titleDebounceTimer.value!.cancel();
          if (titleDirty.value) {
            final value = localTitle.value;
            unawaited(
              client.request(
                GNativeEditor_UpdateDocument_MutationReq(
                  (b) => b.vars.input
                    ..documentId = document.id
                    ..title = Value.present(value.isEmpty ? null : value),
                ),
              ),
            );
          }
        }
        if (subtitleDebounceTimer.value != null) {
          subtitleDebounceTimer.value!.cancel();
          if (subtitleDirty.value) {
            final value = localSubtitle.value;
            unawaited(
              client.request(
                GNativeEditor_UpdateDocument_MutationReq(
                  (b) => b.vars.input
                    ..documentId = document.id
                    ..subtitle = Value.present(value.isEmpty ? null : value),
                ),
              ),
            );
          }
        }
        syncManager.value?.dispose();
        syncManager.value = null;
        editor.value?.dispose();
      };
    }, [document?.id]);

    useEffect(() {
      final currentEditor = editor.value;
      if (currentEditor == null || currentEditor.isDisposed) {
        return null;
      }

      final theme = getEditorTheme(brightness);
      currentEditor.dispatch({
        'type': 'setTheme',
        'theme': {'colors': theme},
      });

      return null;
    }, [editor.value, brightness]);

    useEffect(() {
      final currentEditor = editor.value;
      if (currentEditor == null || currentEditor.isDisposed) {
        return null;
      }

      if (editorController.value != null) {
        return null;
      }

      editorController.value = EditorController(
        editor: currentEditor,
        fontManager: fontManager.value,
        onDocChanged: () => syncManager.value?.handleDocChanged(),
        onExitedDocumentStart: subtitleFocusNode.requestFocus,
      );
      editorControllerReady.value = true;

      return () {
        editorController.value?.dispose();
        editorController.value = null;
      };
    }, [editor.value]);

    useEffect(() {
      final currentEditor = editor.value;
      final documentId = document?.id;
      if (currentEditor == null || currentEditor.isDisposed || documentId == null) {
        return null;
      }

      if (syncManager.value != null) {
        return null;
      }

      syncManager.value = DocumentSyncManager(documentId: documentId, editor: currentEditor, client: client);
      unawaited(syncManager.value!.start());

      return null;
    }, [editor.value, document?.id]);

    useEffect(() {
      final serverTitle = document?.nullableTitle ?? '';
      final serverSubtitle = document?.subtitle ?? '';

      if (titleDirty.value && serverTitle == localTitle.value) {
        titleDirty.value = false;
      }
      if (subtitleDirty.value && serverSubtitle == localSubtitle.value) {
        subtitleDirty.value = false;
      }

      if (!titleDirty.value) {
        localTitle.value = serverTitle;
      }
      if (!subtitleDirty.value) {
        localSubtitle.value = serverSubtitle;
      }
      return null;
    }, [document?.nullableTitle, document?.subtitle]);

    void saveTitle(String documentId, String value) {
      unawaited(
        client.request(
          GNativeEditor_UpdateDocument_MutationReq(
            (b) => b.vars.input
              ..documentId = documentId
              ..title = Value.present(value.isEmpty ? null : value),
          ),
        ),
      );
    }

    void saveSubtitle(String documentId, String value) {
      unawaited(
        client.request(
          GNativeEditor_UpdateDocument_MutationReq(
            (b) => b.vars.input
              ..documentId = documentId
              ..subtitle = Value.present(value.isEmpty ? null : value),
          ),
        ),
      );
    }

    void handleTitleChanged(String value) {
      final documentId = document?.id;
      if (documentId == null) {
        return;
      }

      localTitle.value = value;
      titleDirty.value = true;
      titleDebounceTimer.value?.cancel();
      titleDebounceTimer.value = Timer(const Duration(milliseconds: 200), () {
        saveTitle(documentId, value);
      });
    }

    void handleSubtitleChanged(String value) {
      final documentId = document?.id;
      if (documentId == null) {
        return;
      }

      localSubtitle.value = value;
      subtitleDirty.value = true;
      subtitleDebounceTimer.value?.cancel();
      subtitleDebounceTimer.value = Timer(const Duration(milliseconds: 200), () {
        saveSubtitle(documentId, value);
      });
    }

    final isLoading = editor.value == null && error.value == null && document != null;

    Widget buildBody() {
      if (isLoading) {
        return const SizedBox.shrink();
      }

      if (error.value != null) {
        return Center(
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(LucideLightIcons.circle_alert, size: 48, color: context.colors.textSubtle),
                const SizedBox(height: 16),
                Text(
                  '에디터를 불러올 수 없습니다',
                  style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600, color: context.colors.textDefault),
                ),
                const SizedBox(height: 8),
                Text(
                  error.value!,
                  style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                  textAlign: TextAlign.center,
                ),
              ],
            ),
          ),
        );
      }

      if (editor.value == null || !editorControllerReady.value || editorController.value == null) {
        return const SizedBox.shrink();
      }

      return LayoutBuilder(
        builder: (context, constraints) {
          return EditorView(
            controller: editorController.value!,
            width: constraints.maxWidth,
            height: constraints.maxHeight,
            title: localTitle.value,
            subtitle: localSubtitle.value,
            onTitleChanged: handleTitleChanged,
            onSubtitleChanged: handleSubtitleChanged,
            titleFocusNode: titleFocusNode,
            subtitleFocusNode: subtitleFocusNode,
          );
        },
      );
    }

    return Screen(
      heading: Heading(
        title: headingTitle,
        backgroundColor: context.colors.surfaceDefault,
        onTap: () => editorController.value?.clearFocus(),
        actions: [
          HeadingAction(
            icon: LucideLightIcons.ellipsis,
            onTap: () async {
              editorController.value?.clearFocus();
              await context.showBottomSheet(
                intercept: true,
                child: AppBottomSheet(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      BottomMenuItem(icon: LucideLightIcons.search, label: '찾기', onTap: () {}),
                      BottomMenuItem(icon: LucideLightIcons.bookmark, label: '북마크', onTap: () {}),
                      BottomMenuItem(icon: LucideLightIcons.spell_check, label: '맞춤법 검사', onTap: () {}),
                      BottomMenuItem(icon: LucideLightIcons.lightbulb, label: 'AI 피드백', onTap: () {}),
                      const Gap(16),
                      HorizontalDivider(color: context.colors.borderDefault),
                      const Gap(16),
                      BottomMenuItem(
                        icon: LucideLightIcons.info,
                        label: '정보',
                        onTap: () async {
                          if (document == null) {
                            return;
                          }
                          final characterCounts = editor.value?.getCharacterCounts();
                          await context.showBottomSheet(
                            intercept: true,
                            child: _DocumentInfoBottomSheet(
                              slug: data.entity.slug,
                              client: client,
                              characterCounts: characterCounts,
                            ),
                          );
                        },
                      ),
                      BottomMenuItem(
                        icon: LucideLightIcons.settings,
                        label: '본문 설정',
                        onTap: () async {
                          final controller = editorController.value;
                          if (controller == null) {
                            return;
                          }
                          await context.showBottomSheet(
                            intercept: true,
                            child: NativeEditorBodySettingBottomSheet(controller: controller),
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
                      BottomMenuItem(icon: LucideLightIcons.blend, label: '공유하기', onTap: () {}),
                      BottomMenuItem(
                        icon: LucideLightIcons.copy,
                        label: '복제하기',
                        onTap: () async {
                          if (document == null) {
                            return;
                          }
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
                          if (document == null) {
                            return;
                          }
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
                                  GNativeEditor_DeleteDocument_MutationReq(
                                    (b) => b..vars.input.documentId = document.id,
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
                    ],
                  ),
                ),
              );
            },
          ),
        ],
      ),
      backgroundColor: context.colors.surfaceDefault,
      keyboardDismiss: false,
      responsive: false,
      child: buildBody(),
    );
  }
}

class _DocumentInfoBottomSheet extends HookWidget {
  const _DocumentInfoBottomSheet({required this.slug, required this.client, this.characterCounts});

  final String slug;
  final GraphQLClient client;
  final NativeEditorCharacterCounts? characterCounts;

  @override
  Widget build(BuildContext context) {
    final stream = useMemoized(
      () => client.raw
          .request(
            GNativeEditorScreen_QueryReq(
              (b) => b
                ..vars.slug = slug
                ..fetchPolicy = FetchPolicy.CacheOnly,
            ),
          )
          .where((response) => response.data != null)
          .map((response) => response.data!),
      [slug],
    );
    final snapshot = useStream(stream);

    final document = snapshot.data?.entity.node.when(document: (doc) => doc, orElse: () => null);
    if (document == null) {
      return const SizedBox.shrink();
    }

    final difference = document.characterCountChange.additions - document.characterCountChange.deletions;

    return AppBottomSheet(
      padding: const EdgeInsets.symmetric(horizontal: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            '문서 정보',
            style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700, color: context.colors.textSubtle),
          ),
          const Gap(12),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('최초 생성 시각', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${document.createdAt.toLocal().yyyyMMdd} ${document.createdAt.toLocal().Hm}',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('마지막 수정 시각', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${document.updatedAt.toLocal().yyyyMMdd} ${document.updatedAt.toLocal().Hm}',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          const Gap(32),
          Text(
            '본문 정보',
            style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700, color: context.colors.textSubtle),
          ),
          const Gap(12),
          Row(
            spacing: 4,
            children: [
              Icon(LucideLightIcons.type_, size: 15, color: context.colors.textSubtle),
              Text(
                '글자 수',
                style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          const Gap(8),
          _CharacterCountRow(
            label: '공백 포함',
            docCount: characterCounts?.docWithWhitespace ?? 0,
            selectionCount: characterCounts?.selectionWithWhitespace ?? 0,
          ),
          _CharacterCountRow(
            label: '공백 미포함',
            docCount: characterCounts?.docWithoutWhitespace ?? 0,
            selectionCount: characterCounts?.selectionWithoutWhitespace ?? 0,
          ),
          _CharacterCountRow(
            label: '공백/부호 미포함',
            docCount: characterCounts?.docWithoutWhitespaceAndPunctuation ?? 0,
            selectionCount: characterCounts?.selectionWithoutWhitespaceAndPunctuation ?? 0,
          ),
          const Gap(12),
          Row(
            spacing: 4,
            children: [
              Icon(LucideLightIcons.goal, size: 15, color: context.colors.textSubtle),
              Text(
                '오늘의 기록',
                style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          const Gap(8),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text('변화량', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              ),
              if (difference == 0)
                Text(
                  '없음',
                  style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
                )
              else ...[
                Icon(difference >= 0 ? LucideLightIcons.trending_up : LucideLightIcons.trending_down, size: 14),
                const Gap(4),
                Text(
                  '${difference >= 0 ? '+' : '-'}${difference.abs().comma}자',
                  style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
                ),
              ],
            ],
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('입력한 글자', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${document.characterCountChange.additions.comma}자',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('지운 글자', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${document.characterCountChange.deletions.comma}자',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _CharacterCountRow extends StatelessWidget {
  const _CharacterCountRow({required this.label, required this.docCount, required this.selectionCount});

  final String label;
  final int docCount;
  final int selectionCount;

  @override
  Widget build(BuildContext context) {
    final hasSelection = selectionCount > 0;

    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(label, style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
        Text(
          hasSelection ? '${selectionCount.comma}자 / ${docCount.comma}자' : '${docCount.comma}자',
          style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
        ),
      ],
    );
  }
}
