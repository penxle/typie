import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:auto_route/auto_route.dart';
import 'package:ferry/ferry.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/__generated__/delete_document_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/document_note_query.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.req.gql.dart';
import 'package:typie/screens/native_editor/auto_discard.dart';
import 'package:typie/screens/native_editor/context.dart';
import 'package:typie/screens/native_editor/init.dart';
import 'package:typie/screens/native_editor/limit.dart';
import 'package:typie/screens/native_editor/note.dart';
import 'package:typie/screens/native_editor/sheet/ai_feedback.dart';
import 'package:typie/screens/native_editor/sheet/find_replace.dart';
import 'package:typie/screens/native_editor/sheet/menu.dart';
import 'package:typie/screens/native_editor/sheet/remark.dart';
import 'package:typie/screens/native_editor/sheet/spellcheck.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/fonts.dart';
import 'package:typie/screens/native_editor/state/theme.dart';
import 'package:typie/screens/native_editor/sync/manager.dart';
import 'package:typie/screens/native_editor/sync/persistence.dart';
import 'package:typie/screens/native_editor/sync/selection.dart';
import 'package:typie/screens/native_editor/sync/title.dart';
import 'package:typie/screens/native_editor/view/editor.dart';
import 'package:typie/services/state.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';

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
    final error = useState<String?>(null);
    final pageController = usePageController();
    final drag = useRef<Drag?>(null);
    final mode = useValueNotifier<NativeEditorMode>(NativeEditorMode.editor);
    final currentMode = useValueListenable(mode);
    final autoDiscard = useMemoized(() => AutoDiscardSession.consume(slug), [slug]);

    final document = data.entity.node.when(document: (doc) => doc, orElse: () => null);
    final headingTitle = document?.title ?? '(제목 없음)';

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
      return editorContext.dispose;
    }, []);

    Widget buildEditorBody() {
      if (document == null) {
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

      return EditorScope(
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
          ),
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
                  heading: Heading(
                    titleIcon: document?.type == GDocumentType.TEMPLATE
                        ? LucideLightIcons.layout_template
                        : LucideLightIcons.file,
                    title: headingTitle,
                    backgroundColor: context.colors.surfaceDefault,
                    onTap: () => editorContext.controller?.clearFocus(),
                    actions: [
                      HeadingAction(
                        icon: LucideLightIcons.ellipsis,
                        onTap: () async {
                          editorContext.controller?.clearFocus();
                          if (document == null) {
                            return;
                          }
                          await context.showBottomSheet(
                            intercept: true,
                            child: MenuSheet(
                              data: data,
                              document: document,
                              client: client,
                              editor: editorContext.editor,
                              editorController: editorContext.controller,
                              onOpenFindReplace: () async {
                                final controller = editorContext.controller;
                                if (controller == null) {
                                  return;
                                }
                                await context.showBottomSheet(
                                  intercept: true,
                                  overlayOpacity: 0.05,
                                  dismissKeyboardOnTap: false,
                                  child: FindReplaceSheet(controller: controller),
                                );
                              },
                              onOpenRemark: () async {
                                final controller = editorContext.controller;
                                if (controller == null) {
                                  return;
                                }
                                await context.showBottomSheet(
                                  intercept: true,
                                  overlayOpacity: 0.05,
                                  resizeToAvoidBottomInset: true,
                                  child: RemarkBottomSheet(controller: controller, client: client, userId: data.me!.id),
                                );
                              },
                              onOpenSpellcheck: () async {
                                final controller = editorContext.controller;
                                final currentEditor = editorContext.editor;
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
                                final controller = editorContext.controller;
                                final currentEditor = editorContext.editor;
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
                              onOpenRelatedNotes: () async {
                                editorContext.controller?.clearFocus();
                                await pageController.animateToPage(
                                  1,
                                  duration: const Duration(milliseconds: 300),
                                  curve: Curves.easeInOut,
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
    super.key,
  });

  final String slug;
  final GNativeEditorScreen_QueryData data;
  final GraphQLClient client;
  final ValueNotifier<String?> error;
  final VoidCallback onEdited;

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

      final theme = getEditorTheme(brightness);

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
      onEdited();
      titleSync.value?.handleTitleChanged(value);
      localTitle.value = value;
    }

    void handleSubtitleChanged(String value) {
      onEdited();
      titleSync.value?.handleSubtitleChanged(value);
      localSubtitle.value = value;
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
