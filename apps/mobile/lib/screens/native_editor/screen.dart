import 'dart:async';
import 'dart:convert';
import 'dart:ui' as ui;

import 'package:auto_route/auto_route.dart';
import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.req.gql.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';

import 'theme.dart';

const _fontCdnBase = 'https://cdn.typie.net/fonts/editor';

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

          app.value = await _initApplication();
          editor.value = app.value!.createEditor(scaleFactor, snapshot: snapshot)
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
        app.value?.dispose();
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
      child: _buildBody(context, isLoading: isLoading, error: error.value, editor: editor.value),
    );
  }

  Widget _buildBody(
    BuildContext context, {
    required bool isLoading,
    required String? error,
    required NativeEditor? editor,
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
        return _EditorView(editor: editor, width: constraints.maxWidth, height: constraints.maxHeight);
      },
    );
  }
}

const _pageGap = 24.0;

class _EditorView extends HookWidget {
  const _EditorView({required this.editor, required this.width, required this.height});

  final NativeEditor editor;
  final double width;
  final double height;

  @override
  Widget build(BuildContext context) {
    final layout = useState<_LayoutInfo?>(null);
    final renderVersion = useState<Object>(Object());
    final lastSize = useRef<(double, double, double)?>(null);
    final tickerProvider = useSingleTickerProvider();

    useEffect(() {
      void onTick(Duration elapsed) {
        if (editor.isDisposed) {
          return;
        }

        final scaleFactor = ui.PlatformDispatcher.instance.views.first.devicePixelRatio;
        final currentSize = (width, height, scaleFactor);

        if (lastSize.value != currentSize) {
          lastSize.value = currentSize;
          editor.dispatch({'type': 'resize', 'width': width, 'height': height, 'scaleFactor': scaleFactor});
        }

        final cmds = editor.tick();
        if (cmds != null) {
          for (final cmd in cmds) {
            switch (cmd) {
              case {
                'type': 'layoutChanged',
                'pageCount': final int pageCount,
                'layoutMode': final Map<String, dynamic> layoutMode,
                'pageHeights': final List<dynamic> pageHeights,
              }:
                layout.value = _LayoutInfo(
                  pageCount: pageCount,
                  isPaginated: layoutMode['type'] == 'paginated',
                  pageHeights: pageHeights.cast<num>().map((e) => e.toDouble()).toList(),
                );
              case {'type': 'renderRequired'}:
                renderVersion.value = Object();
            }
          }
        }

        unawaited(
          SchedulerBinding.instance.scheduleTask(() {
            if (!editor.isDisposed) {
              editor.flush();
            }
          }, Priority.idle),
        );
      }

      final ticker = tickerProvider.createTicker(onTick)..start();
      return ticker.dispose;
    }, []);

    final currentLayout = layout.value;
    if (currentLayout == null) {
      return const Center(child: CircularProgressIndicator());
    }

    return ListView.builder(
      itemCount: currentLayout.pageCount,
      cacheExtent: 1000,
      itemBuilder: (context, index) {
        final isLast = index == currentLayout.pageCount - 1;
        final gap = currentLayout.isPaginated && !isLast ? _pageGap : 0.0;
        final pageHeight = currentLayout.pageHeights.elementAtOrNull(index);
        return _PageItem(
          key: ValueKey(index),
          pageIndex: index,
          editor: editor,
          renderVersion: renderVersion.value,
          bottomGap: gap,
          placeholderHeight: pageHeight,
        );
      },
    );
  }
}

class _LayoutInfo {
  const _LayoutInfo({required this.pageCount, required this.isPaginated, required this.pageHeights});

  final int pageCount;
  final bool isPaginated;
  final List<double> pageHeights;
}

class _PageItem extends HookWidget {
  const _PageItem({
    required this.pageIndex,
    required this.editor,
    required this.renderVersion,
    required this.bottomGap,
    required this.placeholderHeight,
    super.key,
  });

  final int pageIndex;
  final NativeEditor editor;
  final Object renderVersion;
  final double bottomGap;
  final double? placeholderHeight;

  @override
  Widget build(BuildContext context) {
    final image = useState<ui.Image?>(null);

    useEffect(() {
      Future<void> render() async {
        image.value = await _renderPage(editor, pageIndex);
      }

      unawaited(render());
      return null;
    }, [pageIndex, renderVersion]);

    if (image.value != null) {
      return Padding(
        padding: EdgeInsets.only(bottom: bottomGap),
        child: RawImage(image: image.value),
      );
    }

    return Container(
      height: placeholderHeight,
      margin: EdgeInsets.only(bottom: bottomGap),
      child: const Center(child: CircularProgressIndicator()),
    );
  }
}

Future<NativeEditorApplication> _initApplication() async {
  final icuData = await rootBundle.load('assets/native/icu_data.postcard');
  final fontResponse = await Dio().get<List<int>>(
    '$_fontCdnBase/Pretendard-Regular.ttf',
    options: Options(responseType: ResponseType.bytes),
  );

  return NativeEditorApplication()
    ..loadIcuData(icuData.buffer.asUint8List())
    ..registerFont('Pretendard', 400, Uint8List.fromList(fontResponse.data!))
    ..setAvailableFonts({
      'Pretendard': [400, 500, 600, 700],
    });
}

Future<ui.Image> _renderPage(NativeEditor editor, int pageIndex) async {
  final result = editor.renderPage(pageIndex);
  final buffer = await ui.ImmutableBuffer.fromUint8List(result.data);
  final descriptor = ui.ImageDescriptor.raw(
    buffer,
    width: result.width,
    height: result.height,
    pixelFormat: ui.PixelFormat.rgba8888,
  );
  final codec = await descriptor.instantiateCodec();
  final frame = await codec.getNextFrame();
  return frame.image;
}
