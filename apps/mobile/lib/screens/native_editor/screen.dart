import 'dart:async';
import 'dart:convert';
import 'dart:ui' as ui;

import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.req.gql.dart';
import 'package:typie/screens/native_editor/fonts.dart';
import 'package:typie/screens/native_editor/theme.dart';
import 'package:typie/screens/native_editor/util/initializer.dart';
import 'package:typie/screens/native_editor/view/editor_view.dart';
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
      builder: (context, client, data) => _Content(data: data),
    );
  }
}

class _Content extends HookWidget {
  const _Content({required this.data});

  final GNativeEditorScreen_QueryData data;

  @override
  Widget build(BuildContext context) {
    final error = useState<String?>(null);
    final app = useRef<NativeEditorApplication?>(null);
    final fontManager = useRef<EditorFontManager?>(null);
    final editor = useState<NativeEditor?>(null);

    final document = data.entity.node.when(document: (doc) => doc, orElse: () => null);
    final title = document?.title ?? '(제목 없음)';
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

    final isLoading = editor.value == null && error.value == null && document != null;

    return Screen(
      heading: Heading(title: title, backgroundColor: context.colors.surfaceDefault),
      backgroundColor: context.colors.surfaceDefault,
      keyboardDismiss: false,
      responsive: false,
      child: _buildBody(
        context,
        isLoading: isLoading,
        error: error.value,
        editor: editor.value,
        fontManager: fontManager.value,
      ),
    );
  }

  Widget _buildBody(
    BuildContext context, {
    required bool isLoading,
    required String? error,
    required NativeEditor? editor,
    required EditorFontManager? fontManager,
  }) {
    if (isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (error != null) {
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
                error,
                style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                textAlign: TextAlign.center,
              ),
            ],
          ),
        ),
      );
    }

    if (editor == null) {
      return const Center(child: CircularProgressIndicator());
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        return EditorView(
          editor: editor,
          fontManager: fontManager,
          width: constraints.maxWidth,
          height: constraints.maxHeight,
        );
      },
    );
  }
}
