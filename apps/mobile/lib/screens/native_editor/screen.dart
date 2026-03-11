import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:auto_route/auto_route.dart';
import 'package:ferry/ferry.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/modals/share.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/native_editor/__generated__/delete_document_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/document_note_query.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/duplicate_document_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/update_document_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/update_document_type_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/user_usage_update_stream.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/view_entity_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/auto_discard.dart';
import 'package:typie/screens/native_editor/context.dart';
import 'package:typie/screens/native_editor/heading.dart';
import 'package:typie/screens/native_editor/init.dart';
import 'package:typie/screens/native_editor/note.dart';
import 'package:typie/screens/native_editor/sheet/ai_feedback.dart';
import 'package:typie/screens/native_editor/sheet/export.dart';
import 'package:typie/screens/native_editor/sheet/find_replace.dart';
import 'package:typie/screens/native_editor/sheet/info.dart';
import 'package:typie/screens/native_editor/sheet/remark.dart';
import 'package:typie/screens/native_editor/sheet/settings.dart';
import 'package:typie/screens/native_editor/sheet/spellcheck.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/fonts.dart';
import 'package:typie/screens/native_editor/state/theme.dart';
import 'package:typie/screens/native_editor/sync/manager.dart';
import 'package:typie/screens/native_editor/sync/persistence.dart';
import 'package:typie/screens/native_editor/sync/selection.dart';
import 'package:typie/screens/native_editor/sync/title.dart';
import 'package:typie/screens/native_editor/view/editor.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/services/state.dart';
import 'package:typie/widgets/plan_upgrade_bottom_sheet.dart';
import 'package:typie/widgets/screen.dart';
import 'package:url_launcher/url_launcher.dart';

enum NativeEditorMode { editor, note }

@RoutePage()
class NativeEditorScreen extends StatelessWidget {
  const NativeEditorScreen({required this.slug, super.key});

  final String slug;

  @override
  Widget build(BuildContext context) {
    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceDefault,
      operation: GNativeEditorScreen_QueryReq(
        (b) => b
          ..vars.slug = slug
          ..fetchPolicy = FetchPolicy.CacheAndNetwork,
      ),
      builder: (context, client, data) => _Content(slug: slug, data: data, client: client),
    );
  }
}

class _Content extends HookWidget {
  const _Content({required this.slug, required this.data, required this.client});

  final String slug;
  final GNativeEditorScreen_QueryData data;
  final GraphQLClient client;

  @override
  Widget build(BuildContext context) {
    final editorContext = useMemoized(EditorContext.new);
    final headerKey = useMemoized(GlobalKey.new);
    final pref = useService<Pref>();
    final error = useState<String?>(null);
    final pageController = usePageController();
    final drag = useRef<Drag?>(null);
    final mode = useValueNotifier<NativeEditorMode>(NativeEditorMode.editor);
    final currentMode = useValueListenable(mode);
    final autoDiscard = useMemoized(() => AutoDiscardSession.consume(slug), [slug]);

    final document = data.entity.node.when(document: (doc) => doc, orElse: () => null);

    void markEdited() {
      autoDiscard.markEdited();
    }

    useEffect(() {
      if (document != null) {
        final snapshotValue = document.snapshot.value;
        editorContext
          ..serverSnapshot = snapshotValue.isNotEmpty ? Uint8List.fromList(base64Decode(snapshotValue)) : null
          ..serverVersion = document.version.value
          ..serverGeneration = document.generation;
      }
      return null;
    }, [document?.snapshot.value, document?.version.value]);

    useEffect(() {
      unawaited(
        client.request(GNativeEditorScreen_ViewEntity_MutationReq((b) => b..vars.input.entityId = data.entity.id)),
      );
      return null;
    }, [data.entity.id]);

    useEffect(() {
      return editorContext.dispose;
    }, []);

    Widget buildEditorBody() {
      if (document == null) {
        return const SizedBox.shrink();
      }

      Widget child;

      if (error.value != null) {
        child = Center(
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
      } else {
        child = EditorScope(
          editorContext: editorContext,
          child: ValueListenableBuilder<int>(
            valueListenable: editorContext.resetKey,
            builder: (context, resetKey, _) => _EditorContent(
              key: ValueKey(resetKey),
              slug: slug,
              data: data,
              client: client,
              error: error,
              onEdited: markEdited,
              headerKey: headerKey,
            ),
          ),
        );
      }

      final topInset = MediaQuery.paddingOf(context).top;

      return Stack(
        children: [
          Positioned.fill(child: child),
          Positioned(
            top: 0,
            left: 0,
            right: 0,
            height: topInset + 72,
            child: IgnorePointer(
              child: DecoratedBox(
                decoration: BoxDecoration(
                  gradient: LinearGradient(
                    begin: Alignment.topCenter,
                    end: Alignment.bottomCenter,
                    colors: [
                      context.colors.surfaceDefault.withValues(alpha: 0.98),
                      context.colors.surfaceDefault.withValues(alpha: 0.86),
                      context.colors.surfaceDefault.withValues(alpha: 0.42),
                      context.colors.surfaceDefault.withValues(alpha: 0),
                    ],
                    stops: const [0, 0.32, 0.72, 1],
                  ),
                ),
              ),
            ),
          ),
        ],
      );
    }

    Future<bool> ensureFullAccess(String message) async {
      if (data.me!.subscription != null) {
        return true;
      }

      final result = await context.showBottomSheet<PlanUpgradeResult>(
        intercept: true,
        child: PlanUpgradeBottomSheet(message: message),
      );

      if (result == PlanUpgradeResult.trialStarted) {
        unawaited(client.refetch(GNativeEditorScreen_QueryReq((b) => b.vars.slug = slug)));
      } else if (result == PlanUpgradeResult.upgrade) {
        if (context.mounted) {
          await context.router.popAndPush(const EnrollPlanRoute());
        }
      }

      return false;
    }

    Future<void> openFindReplace() async {
      final controller = editorContext.controller;
      if (controller == null) {
        return;
      }

      await context.showBottomSheet(
        intercept: true,
        overlayOpacity: 0.05,
        dismissKeyboardOnTap: false,
        heightNotifier: controller.sheetBottomInset,
        child: FindReplaceSheet(controller: controller),
      );
    }

    Future<void> openRemark() async {
      final controller = editorContext.controller;
      if (controller == null) {
        return;
      }

      try {
        await context.showBottomSheet(
          intercept: true,
          overlayOpacity: 0.05,
          heightNotifier: controller.sheetBottomInset,
          child: RemarkBottomSheet(controller: controller, client: client, userId: data.me!.id),
        );
      } finally {
        if (!controller.isDisposed) {
          controller.remarkHighlightTarget.value = null;
        }
      }
    }

    Future<void> openSpellcheck() async {
      final controller = editorContext.controller;
      final currentEditor = editorContext.editor;
      final doc = document;
      if (controller == null || currentEditor == null || doc == null) {
        return;
      }

      final hasAccess = await ensureFullAccess('맞춤법 검사는 FULL ACCESS 플랜에서 사용할 수 있어요.');
      if (!hasAccess || !context.mounted) {
        return;
      }

      await context.showBottomSheet(
        intercept: true,
        overlayOpacity: 0.05,
        heightNotifier: controller.sheetBottomInset,
        child: SpellcheckSheet(controller: controller, editor: currentEditor, documentId: doc.id, client: client),
      );
    }

    Future<void> openAiFeedback() async {
      final controller = editorContext.controller;
      final currentEditor = editorContext.editor;
      final doc = document;
      if (controller == null || currentEditor == null || doc == null) {
        return;
      }

      final hasAccess = await ensureFullAccess('AI 피드백은 FULL ACCESS 플랜에서 사용할 수 있어요.');
      if (!hasAccess || !context.mounted) {
        return;
      }

      final aiOptIn = (data.me!.preferences.asMap['aiOptIn'] as bool?) ?? false;

      await context.showBottomSheet(
        intercept: true,
        overlayOpacity: 0.05,
        heightNotifier: controller.sheetBottomInset,
        child: AiFeedbackSheet(
          controller: controller,
          editor: currentEditor,
          documentId: doc.id,
          client: client,
          aiOptIn: aiOptIn,
        ),
      );
    }

    Future<void> openRelatedNotes() async {
      editorContext.controller?.clearFocus();
      await pageController.animateToPage(1, duration: const Duration(milliseconds: 300), curve: Curves.easeInOut);
    }

    Future<void> openInfo() async {
      final characterCounts = editorContext.editor?.getCharacterCounts();
      await context.showBottomSheet(
        intercept: true,
        child: InfoSheet(slug: data.entity.slug, client: client, characterCounts: characterCounts),
      );
    }

    Future<void> openSettings() async {
      final controller = editorContext.controller;
      if (controller == null) {
        return;
      }

      await context.showBottomSheet(intercept: true, child: SettingsSheet(controller: controller));
    }

    Future<void> toggleLocked() async {
      final doc = document;
      if (doc == null) {
        return;
      }

      unawaited(
        client.request(
          GNativeEditor_UpdateDocument_MutationReq(
            (b) => b.vars.input
              ..documentId = doc.id
              ..locked = Value.present(!doc.locked),
          ),
        ),
      );

      if (context.mounted) {
        context.toast(ToastType.success, doc.locked ? '편집 잠금이 해제되었어요.' : '편집 잠금이 설정되었어요.');
      }
    }

    Future<void> openExport() async {
      final doc = document;
      if (doc == null) {
        return;
      }

      await context.showBottomSheet(
        intercept: true,
        child: ExportSheet(
          documentId: doc.id,
          client: client,
          layout: editorContext.controller?.state.layout,
          hasSubscription: data.me?.subscription != null,
        ),
      );
    }

    Future<void> openInSpace() async {
      await launchUrl(Uri.parse(data.entity.url), mode: LaunchMode.externalApplication);
    }

    Future<void> openShare() async {
      await context.showBottomSheet(intercept: true, child: ShareBottomSheet(entityIds: [data.entity.id]));
    }

    Future<void> duplicateDocument() async {
      final doc = document;
      if (doc == null) {
        return;
      }

      final res = await client.request(
        GNativeEditor_DuplicateDocument_MutationReq((b) => b..vars.input.documentId = doc.id),
      );
      if (context.mounted) {
        await context.router.popAndPush(NativeEditorRoute(slug: res.duplicateDocument.entity.slug));
      }
    }

    Future<void> toggleDocumentType() async {
      final doc = document;
      if (doc == null) {
        return;
      }

      final isToTemplate = doc.type != GDocumentType.TEMPLATE;

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
                  ..vars.input.documentId = doc.id
                  ..vars.input.type = isToTemplate ? GDocumentType.TEMPLATE : GDocumentType.NORMAL,
              ),
            );
            if (context.mounted) {
              await context.router.maybePop();
            }
          },
        ),
      );
    }

    Future<void> deleteDocument() async {
      final doc = document;
      if (doc == null) {
        return;
      }

      await context.showModal(
        intercept: true,
        child: ConfirmModal(
          title: '문서 삭제',
          message: '"${doc.title}" 문서를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.',
          confirmText: '삭제하기',
          confirmTextColor: context.colors.textBright,
          confirmBackgroundColor: context.colors.accentDanger,
          onConfirm: () async {
            await client.request(GNativeEditor_DeleteDocument_MutationReq((b) => b..vars.input.documentId = doc.id));
            if (context.mounted) {
              await context.router.maybePop();
            }
          },
        ),
      );
    }

    return PopScope(
      canPop: currentMode == NativeEditorMode.editor,
      onPopInvokedWithResult: (didPop, result) {
        if (didPop) {
          final doc = document;
          if (!autoDiscard.takeShouldDeleteOnClose() || doc == null) {
            return;
          }
          unawaited(() async {
            try {
              final noteResp = await client.request(GDocumentNote_QueryReq((b) => b..vars.entityId = data.entity.id));
              if (noteResp.entity.notes.isNotEmpty) {
                return;
              }
              await client.request(GNativeEditor_DeleteDocument_MutationReq((b) => b..vars.input.documentId = doc.id));
            } catch (_) {}
          }());
          return;
        }

        if (currentMode == NativeEditorMode.note) {
          unawaited(
            pageController.animateToPage(0, duration: const Duration(milliseconds: 300), curve: Curves.easeInOut),
          );
          return;
        }
      },
      child: Material(
        color: context.colors.surfaceDefault,
        child: Stack(
          children: [
            PageView(
              controller: pageController,
              physics: const NeverScrollableScrollPhysics(),
              onPageChanged: (value) {
                mode.value = switch (value) {
                  0 => NativeEditorMode.editor,
                  1 => NativeEditorMode.note,
                  _ => throw UnimplementedError(),
                };
              },
              children: [
                Screen(
                  heading: NativeEditorHeading(
                    key: headerKey,
                    editorContext: editorContext,
                    documentType: document?.type,
                    toolsPane: NativeEditorToolsPopoverPane(
                      editorContext: editorContext,
                      onOpenFindReplace: openFindReplace,
                      onOpenRelatedNotes: openRelatedNotes,
                      onOpenRemark: openRemark,
                      onOpenSpellcheck: openSpellcheck,
                      onOpenAiFeedback: openAiFeedback,
                      onSendInputLog: pref.devMode && editorContext.showInputRecordingSheet != null
                          ? () async {
                              editorContext.showInputRecordingSheet?.call();
                            }
                          : null,
                    ),
                    documentMenuPane: document == null
                        ? const SizedBox.shrink()
                        : NativeEditorDocumentMenuPopoverPane(
                            editorContext: editorContext,
                            data: data,
                            document: document,
                            onOpenInfo: openInfo,
                            onOpenSettings: openSettings,
                            onToggleLocked: toggleLocked,
                            onOpenExport: openExport,
                            onOpenInSpace: openInSpace,
                            onOpenShare: openShare,
                            onDuplicate: duplicateDocument,
                            onToggleDocumentType: toggleDocumentType,
                            onDelete: deleteDocument,
                          ),
                  ),
                  backgroundColor: context.colors.surfaceDefault,
                  keyboardDismiss: false,
                  responsive: false,
                  extendBodyBehindAppBar: true,
                  child: buildEditorBody(),
                ),
                DocumentNote(
                  entityId: data.entity.id,
                  isActive: currentMode == NativeEditorMode.note,
                  onBack: () async {
                    await pageController.animateToPage(
                      0,
                      duration: const Duration(milliseconds: 300),
                      curve: Curves.easeInOut,
                    );
                  },
                ),
              ],
            ),
            Positioned(
              top: 0,
              left: 0,
              right: 0,
              height: MediaQuery.paddingOf(context).top + 52,
              child: GestureDetector(
                onHorizontalDragDown: (details) {
                  drag.value?.cancel();
                  drag.value = null;
                },
                onHorizontalDragStart: (details) {
                  drag.value = pageController.position.drag(
                    DragStartDetails(globalPosition: details.globalPosition, localPosition: details.localPosition),
                    () {},
                  );
                },
                onHorizontalDragUpdate: (details) {
                  drag.value?.update(
                    DragUpdateDetails(
                      globalPosition: details.globalPosition,
                      localPosition: details.localPosition,
                      delta: Offset(details.delta.dx, 0),
                      primaryDelta: details.delta.dx,
                    ),
                  );
                },
                onHorizontalDragEnd: (details) {
                  drag.value?.end(
                    DragEndDetails(
                      velocity: Velocity(pixelsPerSecond: Offset(details.velocity.pixelsPerSecond.dx, 0)),
                      primaryVelocity: details.velocity.pixelsPerSecond.dx,
                    ),
                  );
                  drag.value = null;
                },
                onHorizontalDragCancel: () {
                  drag.value?.cancel();
                  drag.value = null;
                },
                behavior: HitTestBehavior.translucent,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _EditorContent extends HookWidget {
  const _EditorContent({
    required this.slug,
    required this.data,
    required this.client,
    required this.error,
    required this.onEdited,
    required this.headerKey,
    super.key,
  });

  final String slug;
  final GNativeEditorScreen_QueryData data;
  final GraphQLClient client;
  final ValueNotifier<String?> error;
  final VoidCallback onEdited;
  final GlobalKey headerKey;

  @override
  Widget build(BuildContext context) {
    useAutomaticKeepAlive();

    final editorContext = EditorScope.of(context);
    final app = useRef<NativeEditorApplication?>(null);
    final fontManager = useRef<FontManager?>(null);
    final editor = useState<NativeEditor?>(null);
    final editorController = useRef<EditorController?>(null);
    final editorControllerReady = useState(false);
    final syncManager = useRef<SyncManager?>(null);
    final titleSync = useRef<TitleSyncManager?>(null);
    final selectionSync = useRef<SelectionSyncManager?>(null);

    final titleFocusNode = useFocusNode();
    final subtitleFocusNode = useFocusNode();
    final editorReady = useRef(false);

    final localTitle = useState<String>('');
    final localSubtitle = useState<String>('');

    final appState = useService<AppState>();

    final document = data.entity.node.when(document: (doc) => doc, orElse: () => null);
    final initialView = ui.PlatformDispatcher.instance.views.first;
    final initialViewportWidth = initialView.physicalSize.width / initialView.devicePixelRatio;
    final initialViewportHeight = initialView.physicalSize.height / initialView.devicePixelRatio;
    final scaleFactor = initialView.devicePixelRatio;
    final brightness = context.theme.brightness;

    useEffect(() {
      if (document == null) {
        error.value = 'Document not found';
        return null;
      }

      titleSync.value = TitleSyncManager(documentId: document.id, client: client);
      selectionSync.value = SelectionSyncManager(appState: appState, slug: slug);

      selectionSync.value!.setupFocusListeners(titleFocusNode, subtitleFocusNode, () => editorReady.value);

      final rules = data.me!.textReplacements
          .map(
            (item) => item.when(
              textReplacement: (tr) => {
                'id': tr.id,
                'matchPattern': tr.match,
                'substitute': tr.substitute,
                'regex': tr.regex,
              },
              textReplacementPreference: (pref) => pref.state == GTextReplacementState.ACTIVE
                  ? {
                      'id': pref.textReplacement.id,
                      'matchPattern': pref.textReplacement.match,
                      'substitute': pref.textReplacement.substitute,
                      'regex': pref.textReplacement.regex,
                    }
                  : null,
              orElse: () => null,
            ),
          )
          .whereType<Map<String, dynamic>>()
          .toList();
      setTextReplacementRules(rules);

      final availableFonts = <String, List<int>>{
        for (final f in data.me!.documentFontFamilies) f.familyName: f.fonts.map((font) => font.weight).toList(),
      };
      setAvailableFonts(availableFonts);

      Future<void> init() async {
        try {
          final snapshotBase64 = document.snapshot.value;
          final snapshot = snapshotBase64.isNotEmpty ? base64Decode(snapshotBase64) : null;

          final (application, manager) = await getOrInitializeApplication();
          app.value = application;
          fontManager.value = manager;

          final theme = getEditorTheme(brightness);

          manager.fontFamilies = document.fontFamilies
              .map(
                (f) => FontFamily(
                  id: f.id,
                  familyName: f.familyName,
                  displayName: f.displayName,
                  state: f.state.name,
                  fonts: f.fonts
                      .map(
                        (font) => Font(
                          id: font.id,
                          weight: font.weight,
                          subfamilyDisplayName: font.subfamilyDisplayName,
                          url: font.url,
                          state: font.state.name,
                        ),
                      )
                      .toList(),
                ),
              )
              .toList();

          editor.value = application.createEditor(scaleFactor, snapshot: snapshot)
            ..dispatch({
              'type': 'initialize',
              'theme': {'colors': theme},
              'viewportWidth': initialViewportWidth,
              'viewportHeight': initialViewportHeight,
              'scaleFactor': scaleFactor,
            });
          editorContext.editor = editor.value;
        } on EditorException catch (err) {
          error.value = err.message;
        } catch (err) {
          error.value = err.toString();
        }
      }

      unawaited(init());

      return () {
        titleSync.value?.dispose();
        selectionSync.value?.dispose(titleFocusNode, subtitleFocusNode);
        syncManager.value?.dispose();
        syncManager.value = null;
        editor.value?.dispose();
      };
    }, [document?.id]);

    final loadedSnapshot = useRef<String?>(null);

    useEffect(() {
      final currentEditor = editor.value;
      final snapshotValue = document?.snapshot.value;
      if (currentEditor == null || currentEditor.isDisposed || snapshotValue == null || snapshotValue.isEmpty) {
        return null;
      }

      if (loadedSnapshot.value != null && loadedSnapshot.value != snapshotValue) {
        try {
          currentEditor.importUpdates(base64Decode(snapshotValue));
        } on EditorException catch (err) {
          if (!currentEditor.isDisposed) {
            debugPrint('NativeEditorScreen snapshot import skipped: $err');
          }
        }
      }
      loadedSnapshot.value = snapshotValue;

      return null;
    }, [editor.value, document?.snapshot.value]);

    useEffect(() {
      final currentEditor = editor.value;
      if (currentEditor == null || currentEditor.isDisposed) {
        return null;
      }

      final theme = getEditorTheme(brightness);
      try {
        currentEditor.dispatch({
          'type': 'setTheme',
          'theme': {'colors': theme},
        });
      } on EditorException catch (err) {
        if (!currentEditor.isDisposed) {
          debugPrint('NativeEditorScreen theme dispatch skipped: $err');
        }
      }

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
        onDocChanged: () {
          if (editorReady.value) {
            onEdited();
          }
          syncManager.value?.handleDocChanged();
        },
        onExitedDocumentStart: subtitleFocusNode.requestFocus,
        onSelectionChanged: (anchor, head) {
          selectionSync.value?.handleSelectionChanged(
            anchor,
            head,
            () => editorReady.value,
            () => editorController.value?.state.isFocused ?? false,
          );
        },
        onEditorReady: () {
          selectionSync.value?.restore(
            controller: editorController.value,
            titleFocusNode: titleFocusNode,
            subtitleFocusNode: subtitleFocusNode,
          );
          editorReady.value = true;
        },
      );
      editorController.value!.onEditBlocked = (reason) {
        if (!context.mounted) {
          return;
        }
        editorController.value?.clearFocus();
        if (reason == 'locked') {
          context.toast(ToastType.notification, '편집이 잠겨있는 문서예요.');
          return;
        }
        final message = switch (reason) {
          'restrictedText' => '현재 플랜의 최대 입력 가능 글자 수를 초과했어요.\nFULL ACCESS로 업그레이드하고 이어서 작성하세요.',
          'restrictedBlob' => '현재 플랜의 최대 업로드 가능 용량을 초과했어요.\nFULL ACCESS로 업그레이드하고 이어서 업로드하세요.',
          _ => '현재 플랜의 최대 사용량을 초과했어요.\nFULL ACCESS로 업그레이드하고 이어서 작성하세요.',
        };
        unawaited(
          context
              .showBottomSheet<PlanUpgradeResult>(intercept: true, child: PlanUpgradeBottomSheet(message: message))
              .then((result) {
                if (result == PlanUpgradeResult.upgrade && context.mounted) {
                  unawaited(context.router.popAndPush(const EnrollPlanRoute()));
                }
              }),
        );
      };
      editorControllerReady.value = true;
      editorContext.controller = editorController.value;

      return () {
        final staleController = editorController.value;
        editorController.value = null;
        if (staleController != null) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (!staleController.isDisposed) {
              staleController.dispose();
            }
          });
        }
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

      final persistence = LocalPersistence(documentId);

      syncManager.value = SyncManager(
        documentId: documentId,
        editor: currentEditor,
        client: client,
        persistence: persistence,
        editorContext: editorContext,
      );

      Future<void> init() async {
        try {
          final local = await persistence.load();
          if (currentEditor.isDisposed || syncManager.value == null) {
            return;
          }

          if (local != null && local.snapshot != null && persistence.generation == editorContext.serverGeneration) {
            currentEditor.importUpdatesBatch([local.snapshot!, ...local.updates]);
          } else {
            await persistence.clear();
            if (currentEditor.isDisposed) {
              return;
            }
            final snap = currentEditor.export(DocExportMode.snapshot);
            final version = currentEditor.export(DocExportMode.version);
            final serverVersion = document?.version.value ?? '';
            if (snap != null && version != null && serverVersion.isNotEmpty) {
              await persistence.saveSnapshot(
                snap,
                Uint8List.fromList(version),
                generation: editorContext.serverGeneration,
              );
              await persistence.saveCheckpoint(Uint8List.fromList(base64Decode(serverVersion)));
            }
          }

          if (currentEditor.isDisposed || syncManager.value == null) {
            return;
          }
          await syncManager.value!.start();
        } on EditorException catch (err) {
          if (!currentEditor.isDisposed) {
            debugPrint('NativeEditorScreen sync init skipped: $err');
          }
        }
      }

      unawaited(init());

      return null;
    }, [editor.value, document?.id]);

    useEffect(
      () {
        final ctrl = editorController.value;
        if (ctrl == null) {
          return null;
        }

        const defaultMaxCharCount = 200000;
        const defaultMaxBlobSize = 100000000;

        final maxChar = data.me!.subscription?.plan.rule.maxTotalCharacterCount ?? defaultMaxCharCount;
        final maxBlob = data.me!.subscription?.plan.rule.maxTotalBlobSize ?? defaultMaxBlobSize;

        ctrl
          ..restrictedText = maxChar >= 0 && data.me!.usage.totalCharacterCount >= maxChar
          ..restrictedBlob = maxBlob >= 0 && int.parse(data.me!.usage.totalBlobSize.value) >= maxBlob
          ..locked = document?.locked ?? false;

        return null;
      },
      [
        editorController.value,
        data.me?.usage.totalCharacterCount,
        data.me?.usage.totalBlobSize.value,
        data.me?.subscription?.plan.rule.maxTotalCharacterCount,
        data.me?.subscription?.plan.rule.maxTotalBlobSize,
        document?.locked,
      ],
    );

    useEffect(() {
      final subscription = client
          .subscribe(GNativeEditor_UserUsageUpdateStream_SubscriptionReq((b) => b..vars.userId = data.me!.id))
          .listen((_) {});

      return subscription.cancel;
    }, []);

    void syncHeading({String? title, String? subtitle}) {
      if (title != null && editorContext.headingTitle.value != title) {
        editorContext.headingTitle.value = title;
      }
      if (subtitle != null && editorContext.headingSubtitle.value != subtitle) {
        editorContext.headingSubtitle.value = subtitle;
      }
    }

    useEffect(() {
      final ts = titleSync.value;
      if (ts == null) {
        return null;
      }

      ts.updateFromServer(document?.nullableTitle, document?.subtitle);
      localTitle.value = ts.title;
      localSubtitle.value = ts.subtitle;
      final nextTitle = ts.title;
      final nextSubtitle = ts.subtitle;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!context.mounted) {
          return;
        }
        syncHeading(title: nextTitle, subtitle: nextSubtitle);
      });
      return null;
    }, [document?.nullableTitle, document?.subtitle]);

    void handleTitleChanged(String value) {
      onEdited();
      titleSync.value?.handleTitleChanged(value);
      localTitle.value = value;
      syncHeading(title: value);
    }

    void handleSubtitleChanged(String value) {
      onEdited();
      titleSync.value?.handleSubtitleChanged(value);
      localSubtitle.value = value;
      syncHeading(subtitle: value);
    }

    final isLoading = editor.value == null && error.value == null && document != null;

    if (isLoading) {
      return const SizedBox.shrink();
    }

    if (editor.value == null || !editorControllerReady.value || editorController.value == null) {
      return const SizedBox.shrink();
    }

    return EditorView(
      controller: editorController.value!,
      title: localTitle.value,
      subtitle: localSubtitle.value,
      headerKey: headerKey,
      onTitleChanged: handleTitleChanged,
      onSubtitleChanged: handleSubtitleChanged,
      titleFocusNode: titleFocusNode,
      subtitleFocusNode: subtitleFocusNode,
      documentTemplates: data.entity.site.documentTemplates.toList(),
      assets: document?.assets.toList(),
      client: client,
    );
  }
}
