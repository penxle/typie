import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/update_document_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/init.dart';
import 'package:typie/screens/native_editor/sheet/menu.dart';
import 'package:typie/screens/native_editor/state/fonts.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/state/theme.dart';
import 'package:typie/screens/native_editor/sync/manager.dart';
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

    final localTitle = useState<String>('');
    final localSubtitle = useState<String>('');
    final titleDirty = useState<bool>(false);
    final subtitleDirty = useState<bool>(false);
    final titleFocusNode = useFocusNode();
    final subtitleFocusNode = useFocusNode();
    final titleDebounceTimer = useRef<Timer?>(null);
    final subtitleDebounceTimer = useRef<Timer?>(null);
    final selectionDebounceTimer = useRef<Timer?>(null);
    final editorReady = useRef(false);

    final appState = useService<AppState>();

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
        selectionDebounceTimer.value?.cancel();
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
          if (!editorReady.value || editorController.value?.state.isFocused != true) {
            return;
          }
          selectionDebounceTimer.value?.cancel();
          selectionDebounceTimer.value = Timer(const Duration(milliseconds: 150), () {
            if (!editorReady.value || editorController.value?.state.isFocused != true) {
              return;
            }
            _saveSelectionData(
              appState: appState,
              slug: slug,
              data: jsonEncode({
                'selection': {
                  'anchor': {'nodeId': anchor['nodeId'], 'offset': anchor['offset'], 'affinity': anchor['affinity']},
                  'head': {'nodeId': head['nodeId'], 'offset': head['offset'], 'affinity': head['affinity']},
                },
              }),
            );
          });
        },
        onEditorReady: () {
          _restoreSelection(
            appState: appState,
            slug: slug,
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
      void onTitleFocusChange() {
        if (titleFocusNode.hasFocus && editorReady.value) {
          selectionDebounceTimer.value?.cancel();
          _saveSelectionData(appState: appState, slug: slug, data: jsonEncode({'type': 'element', 'element': 'title'}));
        }
      }

      void onSubtitleFocusChange() {
        if (subtitleFocusNode.hasFocus && editorReady.value) {
          selectionDebounceTimer.value?.cancel();
          _saveSelectionData(
            appState: appState,
            slug: slug,
            data: jsonEncode({'type': 'element', 'element': 'subtitle'}),
          );
        }
      }

      titleFocusNode.addListener(onTitleFocusChange);
      subtitleFocusNode.addListener(onSubtitleFocusChange);
      return () {
        titleFocusNode.removeListener(onTitleFocusChange);
        subtitleFocusNode.removeListener(onSubtitleFocusChange);
      };
    }, []);

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

void _saveSelectionData({required AppState appState, required String slug, required String data}) {
  unawaited(
    appState.setSerializedDocumentSelection(slug, data).catchError((Object e) {
      if (e is FileSystemException && e.osError?.errorCode == 28) {
        return null;
      }
      return Future<void>.error(e);
    }),
  );
}

void _restoreSelection({
  required AppState appState,
  required String slug,
  required EditorController? controller,
  required FocusNode titleFocusNode,
  required FocusNode subtitleFocusNode,
}) {
  final saved = appState.getSerializedDocumentSelection(slug);
  if (saved == null) {
    titleFocusNode.requestFocus();
    return;
  }

  try {
    final data = jsonDecode(saved) as Map<String, dynamic>;

    if (data['type'] == 'element') {
      final element = data['element'] as String;
      if (element == 'title') {
        titleFocusNode.requestFocus();
      } else if (element == 'subtitle') {
        subtitleFocusNode.requestFocus();
      }
      return;
    }

    final selection = data['selection'] as Map<String, dynamic>?;
    if (selection != null && controller != null) {
      final savedAnchor = selection['anchor'] as Map<String, dynamic>;
      final savedHead = selection['head'] as Map<String, dynamic>;
      controller
        ..dispatch({
          'type': 'setSelection',
          'anchorNodeId': savedAnchor['nodeId'],
          'anchorOffset': savedAnchor['offset'],
          'anchorAffinity': savedAnchor['affinity'],
          'headNodeId': savedHead['nodeId'],
          'headOffset': savedHead['offset'],
          'headAffinity': savedHead['affinity'],
        })
        ..requestFocus();
    }
  } catch (err) {
    titleFocusNode.requestFocus();
  }
}
