import 'dart:async';
import 'dart:convert';
import 'dart:ui' as ui;

import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_lab.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/editor/limit.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.req.gql.dart';
import 'package:typie/screens/native_editor/init.dart';
import 'package:typie/screens/native_editor/sheet/ai_feedback.dart';
import 'package:typie/screens/native_editor/sheet/find_replace.dart';
import 'package:typie/screens/native_editor/sheet/menu.dart';
import 'package:typie/screens/native_editor/sheet/spellcheck.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/fonts.dart';
import 'package:typie/screens/native_editor/state/theme.dart';
import 'package:typie/screens/native_editor/sync/manager.dart';
import 'package:typie/screens/native_editor/sync/selection.dart';
import 'package:typie/screens/native_editor/sync/title.dart';
import 'package:typie/screens/native_editor/view/editor.dart';
import 'package:typie/services/state.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';

@RoutePage()
class NativeEditorScreen extends StatelessWidget {
  const NativeEditorScreen({required this.slug, super.key});

  final String slug;

  @override
  Widget build(BuildContext context) {
    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceDefault,
      operation: GNativeEditorScreen_QueryReq((b) => b.vars.slug = slug),
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
    final error = useState<String?>(null);
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
    final headingTitle = document?.title ?? '(제목 없음)';
    final scaleFactor = ui.PlatformDispatcher.instance.views.first.devicePixelRatio;

    final brightness = context.theme.brightness;

    useEffect(() {
      if (document == null) {
        error.value = 'Document not found';
        return null;
      }

      titleSync.value = TitleSyncManager(documentId: document.id, client: client);
      selectionSync.value = SelectionSyncManager(appState: appState, slug: slug);

      selectionSync.value!.setupFocusListeners(titleFocusNode, subtitleFocusNode, () => editorReady.value);

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
        titleSync.value?.dispose();
        selectionSync.value?.dispose(titleFocusNode, subtitleFocusNode);
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

      syncManager.value = SyncManager(documentId: documentId, editor: currentEditor, client: client);
      unawaited(syncManager.value!.start());

      return null;
    }, [editor.value, document?.id]);

    useEffect(() {
      final ts = titleSync.value;
      if (ts == null) {
        return null;
      }

      ts.updateFromServer(document?.nullableTitle, document?.subtitle);
      localTitle.value = ts.title;
      localSubtitle.value = ts.subtitle;
      return null;
    }, [document?.nullableTitle, document?.subtitle]);

    void handleTitleChanged(String value) {
      titleSync.value?.handleTitleChanged(value);
      localTitle.value = value;
    }

    void handleSubtitleChanged(String value) {
      titleSync.value?.handleSubtitleChanged(value);
      localSubtitle.value = value;
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

      return EditorView(
        controller: editorController.value!,
        title: localTitle.value,
        subtitle: localSubtitle.value,
        onTitleChanged: handleTitleChanged,
        onSubtitleChanged: handleSubtitleChanged,
        titleFocusNode: titleFocusNode,
        subtitleFocusNode: subtitleFocusNode,
      );
    }

    return Screen(
      heading: Heading(
        titleIcon: LucideLabIcons.text_square,
        titleIconColor: context.colors.accentBrand,
        title: headingTitle,
        backgroundColor: context.colors.surfaceDefault,
        onTap: () => editorController.value?.clearFocus(),
        actions: [
          HeadingAction(
            icon: LucideLightIcons.ellipsis,
            onTap: () async {
              editorController.value?.clearFocus();
              if (document == null) {
                return;
              }
              await context.showBottomSheet(
                intercept: true,
                child: MenuSheet(
                  data: data,
                  document: document,
                  client: client,
                  editor: editor.value,
                  editorController: editorController.value,
                  onOpenFindReplace: () async {
                    final controller = editorController.value;
                    if (controller == null) {
                      return;
                    }
                    await context.showBottomSheet(
                      intercept: true,
                      overlayOpacity: 0.05,
                      child: FindReplaceSheet(controller: controller),
                    );
                  },
                  onOpenSpellcheck: () async {
                    final controller = editorController.value;
                    final currentEditor = editor.value;
                    if (controller == null || currentEditor == null) {
                      return;
                    }

                    if (data.me!.subscription == null) {
                      final trialStarted = await context.showBottomSheet<bool>(
                        intercept: true,
                        child: const LimitBottomSheet(type: LimitBottomSheetType.spellCheck),
                      );

                      if (trialStarted ?? false) {
                        unawaited(client.refetch(GNativeEditorScreen_QueryReq((b) => b.vars.slug = slug)));
                      }

                      return;
                    }

                    await context.showBottomSheet(
                      intercept: true,
                      overlayOpacity: 0.05,
                      child: SpellcheckSheet(
                        controller: controller,
                        editor: currentEditor,
                        documentId: document.id,
                        client: client,
                      ),
                    );
                  },
                  onOpenAiFeedback: () async {
                    final controller = editorController.value;
                    final currentEditor = editor.value;
                    if (controller == null || currentEditor == null) {
                      return;
                    }

                    if (data.me!.subscription == null) {
                      final trialStarted = await context.showBottomSheet<bool>(
                        intercept: true,
                        child: const LimitBottomSheet(type: LimitBottomSheetType.aiFeedback),
                      );

                      if (trialStarted ?? false) {
                        unawaited(client.refetch(GNativeEditorScreen_QueryReq((b) => b.vars.slug = slug)));
                      }

                      return;
                    }

                    final aiOptIn = (data.me!.preferences.asMap['aiOptIn'] as bool?) ?? false;

                    await context.showBottomSheet(
                      intercept: true,
                      overlayOpacity: 0.05,
                      child: AiFeedbackSheet(
                        controller: controller,
                        editor: currentEditor,
                        documentId: document.id,
                        client: client,
                        aiOptIn: aiOptIn,
                      ),
                    );
                  },
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
