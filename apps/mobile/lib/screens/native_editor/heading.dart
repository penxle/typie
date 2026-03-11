import 'dart:async';
import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/screens/native_editor/context.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/overlay_heading.dart';
import 'package:typie/widgets/popover/list.dart';
import 'package:typie/widgets/popover/popover.dart';

const _headingPopoverScreenPadding = EdgeInsets.fromLTRB(20, 8, 20, 8);
const _documentMenuHeaderHeight = 44.0;
const _documentMenuHeaderIconInset = 14.0;
const _documentMenuHeaderTextInset = 42.0;
const _documentMenuHeaderIconSize = 18.0;
const _documentMenuHeaderSourceRightInset = 14.0;

List<BoxShadow> _headingControlShadow(BuildContext context) => [
  BoxShadow(color: context.colors.shadowDefault.withValues(alpha: 0.06), offset: const Offset(0, 1), blurRadius: 4),
];

class NativeEditorHeading extends StatelessWidget implements PreferredSizeWidget {
  const NativeEditorHeading({
    required this.editorContext,
    required this.documentType,
    required this.toolsPane,
    required this.documentMenuPane,
    super.key,
  });

  final EditorContext editorContext;
  final GDocumentType? documentType;
  final Widget toolsPane;
  final Widget documentMenuPane;

  @override
  Widget build(BuildContext context) {
    final controlBackgroundColor = context.theme.brightness == Brightness.dark
        ? context.colors.surfaceSubtle
        : context.colors.surfaceDefault;
    final controlShadow = _headingControlShadow(context);

    return AnimatedBuilder(
      animation: Listenable.merge([editorContext.headingTitle, editorContext.headingSubtitle]),
      builder: (context, _) {
        final title = editorContext.headingTitle.value;
        final subtitle = editorContext.headingSubtitle.value;
        final capsule = HeadingCapsuleLabel(
          icon: documentType == GDocumentType.TEMPLATE ? LucideLightIcons.layout_template : LucideLightIcons.file,
          title: title.isEmpty ? '(제목 없음)' : title,
          subtitle: subtitle.isEmpty ? null : subtitle,
          backgroundColor: controlBackgroundColor,
          boxShadow: controlShadow,
          borderRadius: Popover.expandedRadius,
        );

        return OverlayHeadingBar(
          onTap: () => editorContext.controller?.clearFocus(),
          leading: HeadingCircleButton(
            icon: LucideLightIcons.chevron_left,
            backgroundColor: controlBackgroundColor,
            boxShadow: controlShadow,
            useSlotHeight: false,
            onTap: () async {
              editorContext.controller?.clearFocus();
              await context.router.maybePop();
            },
          ),
          center: Popover(
            position: PopoverPosition.bottomCenter,
            screenPadding: _headingPopoverScreenPadding,
            collapsedBorderRadius: Popover.defaultExpandedBorderRadius,
            backgroundColor: controlBackgroundColor,
            borderSide: BorderSide(color: context.colors.borderStrong),
            anchor: capsule,
            pane: documentMenuPane,
          ),
          trailing: Popover(
            screenPadding: _headingPopoverScreenPadding,
            collapsedBorderRadius: BorderRadius.circular(999),
            backgroundColor: controlBackgroundColor,
            borderSide: BorderSide(color: context.colors.borderStrong),
            anchor: HeadingCircleButton(
              icon: LucideLightIcons.panel_right,
              useSlotHeight: false,
              backgroundColor: controlBackgroundColor,
              boxShadow: controlShadow,
            ),
            pane: toolsPane,
          ),
        );
      },
    );
  }

  @override
  Size get preferredSize => const Size.fromHeight(OverlayHeading.height);
}

class NativeEditorToolsPopoverPane extends StatelessWidget {
  const NativeEditorToolsPopoverPane({
    required this.editorContext,
    required this.noteCount,
    required this.onOpenFindReplace,
    required this.onOpenRelatedNotes,
    required this.onOpenRemark,
    required this.onOpenSpellcheck,
    required this.onOpenAiFeedback,
    this.onSendInputLog,
    super.key,
  });

  final EditorContext editorContext;
  final int noteCount;
  final Future<void> Function() onOpenFindReplace;
  final Future<void> Function() onOpenRelatedNotes;
  final Future<void> Function() onOpenRemark;
  final Future<void> Function() onOpenSpellcheck;
  final Future<void> Function() onOpenAiFeedback;
  final Future<void> Function()? onSendInputLog;

  @override
  Widget build(BuildContext context) {
    final controller = editorContext.controller;

    Widget buildPane(int remarkCount) {
      final hasNoteBadge = noteCount > 0;
      final hasRemarkBadge = remarkCount > 0;

      return _HeadingPopoverPane(
        trailingSlotWidth: hasNoteBadge || hasRemarkBadge ? 36 : null,
        entries: [
          _HeadingPopoverEntry(icon: LucideLightIcons.search, label: '찾기', onSelected: onOpenFindReplace),
          _HeadingPopoverEntry(
            icon: LucideLightIcons.sticky_note,
            label: '노트',
            onSelected: onOpenRelatedNotes,
            trailing: hasNoteBadge ? _HeadingPopoverBadge(label: '$noteCount') : null,
          ),
          _HeadingPopoverEntry(
            icon: LucideLightIcons.message_square_text,
            label: '코멘트',
            onSelected: onOpenRemark,
            trailing: hasRemarkBadge ? _HeadingPopoverBadge(label: '$remarkCount') : null,
          ),
          _HeadingPopoverEntry(icon: LucideLightIcons.spell_check, label: '맞춤법 검사', onSelected: onOpenSpellcheck),
          _HeadingPopoverEntry(icon: LucideLightIcons.lightbulb, label: 'AI 피드백', onSelected: onOpenAiFeedback),
          if (onSendInputLog != null)
            _HeadingPopoverEntry(icon: LucideLightIcons.send, label: '입력 로그 보내기', onSelected: onSendInputLog!),
        ],
      );
    }

    if (controller == null) {
      return buildPane(0);
    }

    return ListenableBuilder(
      listenable: controller,
      builder: (context, _) => buildPane(controller.state.remarks.length),
    );
  }
}

class NativeEditorDocumentMenuPopoverPane extends StatelessWidget {
  const NativeEditorDocumentMenuPopoverPane({
    required this.editorContext,
    required this.data,
    required this.document,
    required this.onOpenInfo,
    required this.onOpenSettings,
    required this.onToggleLocked,
    required this.onOpenExport,
    required this.onOpenInSpace,
    required this.onOpenShare,
    required this.onDuplicate,
    required this.onToggleDocumentType,
    required this.onDelete,
    super.key,
  });

  final EditorContext editorContext;
  final GNativeEditorScreen_QueryData data;
  final GNativeEditorScreen_QueryData_entity_node__asDocument document;
  final Future<void> Function() onOpenInfo;
  final Future<void> Function() onOpenSettings;
  final Future<void> Function() onToggleLocked;
  final Future<void> Function() onOpenExport;
  final Future<void> Function() onOpenInSpace;
  final Future<void> Function() onOpenShare;
  final Future<void> Function() onDuplicate;
  final Future<void> Function() onToggleDocumentType;
  final Future<void> Function() onDelete;

  @override
  Widget build(BuildContext context) {
    return _HeadingPopoverPane(
      header: _DocumentMenuPaneHeader(
        editorContext: editorContext,
        sourceIcon: document.type == GDocumentType.TEMPLATE ? LucideLightIcons.layout_template : LucideLightIcons.file,
      ),
      expandToMaxWidth: true,
      entries: [
        _HeadingPopoverEntry(icon: LucideLightIcons.info, label: '정보', onSelected: onOpenInfo),
        _HeadingPopoverEntry(icon: LucideLightIcons.settings, label: '본문 설정', onSelected: onOpenSettings),
        _HeadingPopoverEntry(
          icon: document.locked ? LucideLightIcons.lock_open : LucideLightIcons.lock,
          label: document.locked ? '편집 잠금 해제' : '편집 잠금',
          onSelected: onToggleLocked,
        ),
        _HeadingPopoverEntry(icon: LucideLightIcons.file_down, label: '파일로 내보내기', onSelected: onOpenExport),
        _HeadingPopoverEntry(icon: LucideLightIcons.external_link, label: '스페이스에서 열기', onSelected: onOpenInSpace),
        _HeadingPopoverEntry(
          icon: LucideLightIcons.blend,
          label: '공유하기',
          onSelected: onOpenShare,
          trailing:
              data.entity.visibility == GEntityVisibility.PUBLIC || data.entity.visibility == GEntityVisibility.UNLISTED
              ? _HeadingPopoverBadge(label: data.entity.visibility == GEntityVisibility.PUBLIC ? '공개 중' : '링크 공개 중')
              : null,
        ),
        _HeadingPopoverEntry(icon: LucideLightIcons.copy, label: '복제하기', onSelected: onDuplicate),
        _HeadingPopoverEntry(
          icon: LucideLightIcons.layout_template,
          label: document.type == GDocumentType.TEMPLATE ? '문서로 전환' : '템플릿으로 전환',
          onSelected: onToggleDocumentType,
        ),
        _HeadingPopoverEntry(
          icon: LucideLightIcons.trash_2,
          label: '삭제하기',
          onSelected: onDelete,
          iconColor: context.colors.textDanger,
          labelColor: context.colors.textDanger,
        ),
      ],
    );
  }
}

class _HeadingPopoverEntry {
  const _HeadingPopoverEntry({
    required this.icon,
    required this.label,
    required this.onSelected,
    this.iconColor,
    this.labelColor,
    this.trailing,
  });

  final IconData icon;
  final String label;
  final Future<void> Function() onSelected;
  final Color? iconColor;
  final Color? labelColor;
  final Widget? trailing;
}

class _HeadingPopoverPane extends StatelessWidget {
  const _HeadingPopoverPane({
    required this.entries,
    this.header,
    this.expandToMaxWidth = false,
    this.trailingSlotWidth,
  });

  final List<_HeadingPopoverEntry> entries;
  final Widget? header;
  final bool expandToMaxWidth;
  final double? trailingSlotWidth;

  @override
  Widget build(BuildContext context) {
    final content = Padding(
      padding: const EdgeInsets.all(Popover.panePadding),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (header != null) ...[header!, const SizedBox(height: 6)],
          PopoverList(
            indicatorColor: context.colors.surfaceMuted,
            items: [
              for (final entry in entries)
                PopoverListItem(
                  onSelected: () {
                    Popover.close(context);
                    unawaited(entry.onSelected());
                  },
                  child: _HeadingPopoverPaneItem(
                    icon: entry.icon,
                    label: entry.label,
                    iconColor: entry.iconColor,
                    labelColor: entry.labelColor,
                    trailing: entry.trailing,
                    trailingSlotWidth: trailingSlotWidth,
                  ),
                ),
            ],
          ),
        ],
      ),
    );

    if (expandToMaxWidth) {
      return SizedBox(width: double.infinity, child: content);
    }

    return IntrinsicWidth(child: content);
  }
}

class _DocumentMenuPaneHeader extends StatelessWidget {
  const _DocumentMenuPaneHeader({required this.editorContext, required this.sourceIcon});

  final EditorContext editorContext;
  final IconData sourceIcon;

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: Listenable.merge([editorContext.headingTitle, editorContext.headingSubtitle]),
      builder: (context, _) {
        final title = editorContext.headingTitle.value;
        final subtitle = editorContext.headingSubtitle.value;
        final hasSubtitle = subtitle.isNotEmpty;
        final transition = PopoverPaneTransitionScope.maybeOf(context);
        final progress = (transition?.progress ?? 1).clamp(0.0, 1.0);
        final titleText = title.isEmpty ? '(제목 없음)' : title;
        final titleStyle = TextStyle(
          fontSize: hasSubtitle ? 14 : 15,
          fontWeight: FontWeight.w600,
          height: 1,
          color: context.colors.textDefault,
        );
        final subtitleStyle = TextStyle(
          fontSize: 11,
          fontWeight: FontWeight.w500,
          height: 1,
          color: context.colors.textFaint,
        );

        return SizedBox(
          height: _documentMenuHeaderHeight,
          child: LayoutBuilder(
            builder: (context, constraints) {
              final paneWidth = constraints.maxWidth;
              final anchorContentRect =
                  transition?.anchorContentRect ?? Rect.fromLTWH(0, 0, paneWidth, HeadingCircleButton.controlSize);
              final iconLeft = ui.lerpDouble(
                anchorContentRect.left + _documentMenuHeaderIconInset,
                _documentMenuHeaderIconInset,
                progress,
              )!;
              final iconTop = ui.lerpDouble(
                anchorContentRect.top + (anchorContentRect.height - _documentMenuHeaderIconSize) / 2,
                (_documentMenuHeaderHeight - _documentMenuHeaderIconSize) / 2,
                progress,
              )!;
              final sourceTextLeft = anchorContentRect.left + _documentMenuHeaderTextInset;
              final sourceTextWidth = math.max<double>(
                0,
                anchorContentRect.width - _documentMenuHeaderTextInset - _documentMenuHeaderSourceRightInset,
              );
              final targetTextWidth = math.max<double>(0, paneWidth - _documentMenuHeaderTextInset);
              final textLeft = ui.lerpDouble(sourceTextLeft, _documentMenuHeaderTextInset, progress)!;
              final textWidth = ui.lerpDouble(sourceTextWidth, targetTextWidth, progress)!;

              return Stack(
                children: [
                  Positioned(
                    left: 0,
                    top: 0,
                    width: _documentMenuHeaderTextInset,
                    height: _documentMenuHeaderHeight,
                    child: GestureDetector(
                      behavior: HitTestBehavior.opaque,
                      onTap: () {
                        Popover.close(context);
                      },
                      child: const SizedBox.expand(),
                    ),
                  ),
                  Positioned(
                    left: iconLeft,
                    top: iconTop,
                    width: _documentMenuHeaderIconSize,
                    height: _documentMenuHeaderIconSize,
                    child: IgnorePointer(
                      child: Stack(
                        alignment: Alignment.centerLeft,
                        children: [
                          Opacity(
                            opacity: 1 - progress,
                            child: Icon(sourceIcon, size: 18, color: context.colors.textSubtle),
                          ),
                          Opacity(
                            opacity: progress,
                            child: Icon(LucideLightIcons.x, size: 18, color: context.colors.textSubtle),
                          ),
                        ],
                      ),
                    ),
                  ),
                  Positioned(
                    left: textLeft,
                    top: 0,
                    width: textWidth,
                    height: _documentMenuHeaderHeight,
                    child: Align(
                      alignment: Alignment.centerLeft,
                      child: SizedBox(
                        height: _documentMenuHeaderHeight,
                        child: Column(
                          mainAxisAlignment: MainAxisAlignment.center,
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(titleText, maxLines: 1, overflow: TextOverflow.ellipsis, style: titleStyle),
                            if (hasSubtitle) ...[
                              const SizedBox(height: 1),
                              Text(subtitle, maxLines: 1, overflow: TextOverflow.ellipsis, style: subtitleStyle),
                            ],
                          ],
                        ),
                      ),
                    ),
                  ),
                ],
              );
            },
          ),
        );
      },
    );
  }
}

class _HeadingPopoverPaneItem extends StatelessWidget {
  const _HeadingPopoverPaneItem({
    required this.icon,
    required this.label,
    this.iconColor,
    this.labelColor,
    this.trailing,
    this.trailingSlotWidth,
  });

  final IconData icon;
  final String label;
  final Color? iconColor;
  final Color? labelColor;
  final Widget? trailing;
  final double? trailingSlotWidth;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 42,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Row(
          spacing: 12,
          children: [
            Icon(icon, size: 18, color: iconColor ?? context.colors.textDefault),
            Expanded(
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                softWrap: false,
                style: TextStyle(
                  fontSize: 15,
                  fontWeight: FontWeight.w500,
                  color: labelColor ?? context.colors.textDefault,
                ),
              ),
            ),
            if (trailingSlotWidth != null)
              SizedBox(
                width: trailingSlotWidth,
                child: Align(alignment: Alignment.centerRight, child: trailing),
              )
            else
              ?trailing,
          ],
        ),
      ),
    );
  }
}

class _HeadingPopoverBadge extends StatelessWidget {
  const _HeadingPopoverBadge({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    final squircleBorderRadius = BorderRadius.circular(999);

    return Container(
      constraints: const BoxConstraints(minWidth: 24),
      height: 24,
      child: DecoratedBox(
        decoration: ShapeDecoration(
          shape: RoundedSuperellipseBorder(
            borderRadius: squircleBorderRadius,
            side: BorderSide(color: context.colors.borderStrong),
          ),
        ),
        child: ClipRSuperellipse(
          borderRadius: squircleBorderRadius,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: Center(
              child: Text(
                label,
                maxLines: 1,
                strutStyle: const StrutStyle(fontSize: 12, height: 1, leading: 0, forceStrutHeight: true),
                style: TextStyle(
                  fontSize: 12,
                  fontWeight: FontWeight.w500,
                  height: 1,
                  color: context.colors.textDefault,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
