import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/screens/native_editor/sheet/template.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/values.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';

class DocumentPlaceholder extends StatelessWidget {
  const DocumentPlaceholder({
    required this.controller,
    required this.verticalScrollController,
    required this.horizontalScrollController,
    required this.titleAreaHeight,
    required this.scrollMetricsRevision,
    required this.documentTemplates,
    required this.client,
    required this.displayZoom,
    super.key,
  });

  final EditorController controller;
  final ScrollController verticalScrollController;
  final ScrollController horizontalScrollController;
  final ValueNotifier<double> titleAreaHeight;
  final ValueNotifier<int> scrollMetricsRevision;
  final List<GNativeEditorScreen_QueryData_entity_site_documentTemplates> documentTemplates;
  final GraphQLClient client;
  final ValueNotifier<double> displayZoom;

  static const double _ptToPx = 96 / 72;

  double _resolveNumericAttr(List<Map<String, dynamic>> attrs, String type, String defaultKey) {
    final values = (findAttr(attrs, type)?['values'] as List?)?.whereType<num>().toList() ?? const <num>[];
    return (values.length == 1 ? values[0] : editorDefaultValues[defaultKey] as num).toDouble();
  }

  String _resolveTextAlign(List<Map<String, dynamic>> attrs) {
    final values =
        (findAttr(attrs, 'text_align')?['values'] as List?)?.whereType<String>().toList() ?? const <String>[];
    return values.length == 1 ? values[0] : editorDefaultValues['textAlign'] as String;
  }

  @override
  Widget build(BuildContext context) {
    if (controller.isDisposed) {
      return const SizedBox.shrink();
    }

    return ListenableBuilder(
      listenable: Listenable.merge([controller, titleAreaHeight, displayZoom]),
      builder: (context, _) {
        final placeholder = controller.state.placeholder;
        if (!placeholder.visible ||
            placeholder.x == null ||
            placeholder.y == null ||
            placeholder.width == null ||
            placeholder.width! <= 0) {
          return const SizedBox.shrink();
        }

        if (titleAreaHeight.value <= 0) {
          return const SizedBox.shrink();
        }

        final layout = controller.state.layout;
        if (layout == null) {
          return const SizedBox.shrink();
        }

        final attrs = controller.state.attrs;
        final fontSize = _resolveNumericAttr(attrs, 'font_size', 'fontSize');
        final letterSpacing = _resolveNumericAttr(attrs, 'letter_spacing', 'letterSpacing');
        final lineHeight = _resolveNumericAttr(attrs, 'line_height', 'lineHeight');
        final textAlign = _resolveTextAlign(attrs);
        final fontSizePx = (fontSize / 100) * _ptToPx;
        final textStyle = TextStyle(
          fontSize: fontSizePx,
          height: lineHeight / 100,
          letterSpacing: fontSizePx * (letterSpacing / 100),
          color: context.colors.textDisabled,
        );
        final placeholderTextAlign = switch (textAlign) {
          'center' => TextAlign.center,
          'right' => TextAlign.right,
          'justify' => TextAlign.justify,
          _ => TextAlign.left,
        };
        final placeholderAlignment = switch (textAlign) {
          'center' => Alignment.center,
          'right' => Alignment.centerRight,
          _ => Alignment.centerLeft,
        };

        final geo = ContentGeometry(
          layout: layout,
          pages: controller.state.pages,
          titleAreaHeight: titleAreaHeight.value,
          zoom: displayZoom.value,
        );

        return AnimatedBuilder(
          animation: Listenable.merge([verticalScrollController, horizontalScrollController, scrollMetricsRevision]),
          builder: (context, child) {
            final verticalScroll = resolveScrollOffset(verticalScrollController);
            final horizontalMetrics = resolveHorizontalScrollMetrics(
              controller: horizontalScrollController,
              contentWidth: geo.contentWidth,
              fallbackViewportDimension: MediaQuery.sizeOf(context).width,
            );
            final viewportWidth = horizontalMetrics.viewportDimension;
            final horizontalScroll = horizontalMetrics.scrollOffset;
            final placeholderX = placeholder.x!;
            final placeholderY = placeholder.y!;
            final placeholderWidth = placeholder.width!;

            final top = geo.toDisplayY(placeholderY) + titleAreaHeight.value - verticalScroll;
            final left =
                geo.toDisplayX(placeholderX) +
                geo.contentStartX(viewportWidth: viewportWidth, horizontalScrollOffset: horizontalScroll);
            final zoom = geo.effectiveZoom;

            return Positioned(
              top: top,
              left: left,
              width: placeholderWidth,
              child: Transform.scale(alignment: Alignment.topLeft, scale: zoom, child: child),
            );
          },
          child: IconTheme.merge(
            data: IconThemeData(size: fontSizePx, color: context.colors.textDisabled),
            child: DefaultTextStyle.merge(
              style: textStyle,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  IgnorePointer(child: Text('내용을 입력하거나', textAlign: placeholderTextAlign)),
                  const Gap(4),
                  Align(
                    alignment: placeholderAlignment,
                    child: GestureDetector(
                      onTap: () async {
                        controller.clearFocus();
                        await context.showBottomSheet(
                          intercept: true,
                          child: TemplateSheet(
                            templates: documentTemplates,
                            editor: controller.editor,
                            controller: controller,
                            client: client,
                          ),
                        );
                      },
                      child: const Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [Icon(LucideLightIcons.layout_template), Gap(4), Text('템플릿 불러오기')],
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}
