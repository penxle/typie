import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/values.dart';
import 'package:typie/widgets/tappable.dart';

class _NoteFoldClipper extends CustomClipper<Path> {
  @override
  Path getClip(Size size) {
    final path = Path();
    const foldSize = 12.0;

    path
      ..moveTo(0, 0)
      ..lineTo(size.width, 0)
      ..lineTo(size.width, size.height - foldSize)
      ..lineTo(size.width - foldSize, size.height)
      ..lineTo(0, size.height)
      ..close();

    return path;
  }

  @override
  bool shouldReclip(CustomClipper<Path> oldClipper) => false;
}

class _NoteTriangleClipper extends CustomClipper<Path> {
  @override
  Path getClip(Size size) {
    final path = Path()
      ..moveTo(0, 0)
      ..lineTo(size.width, 0)
      ..lineTo(0, size.height)
      ..close();
    return path;
  }

  @override
  bool shouldReclip(CustomClipper<Path> oldClipper) => false;
}

class NoteCard extends StatelessWidget {
  const NoteCard({
    super.key,
    required this.color,
    required this.index,
    required this.controller,
    required this.focusNode,
    required this.isExpanded,
    required this.onExpand,
    required this.onUpdateContent,
    this.footer,
  });

  final String color;
  final int index;
  final TextEditingController? controller;
  final FocusNode? focusNode;
  final bool isExpanded;
  final VoidCallback onExpand;
  final void Function(String) onUpdateContent;
  final Widget? footer;

  @override
  Widget build(BuildContext context) {
    Color getNoteBackgroundColor(String color) {
      final backgroundColors = editorValues['textBackgroundColor']!;

      final colorMap = backgroundColors.firstWhere(
        (item) => item['value'] == color,
        orElse: () => {'color': Colors.transparent},
      );

      final colorFunc = colorMap['color'] as Color Function(BuildContext)?;
      if (colorFunc != null) {
        return colorFunc(context);
      }

      return context.colors.prosemirrorWhite;
    }

    final backgroundColor = getNoteBackgroundColor(color);

    return Stack(
      children: [
        ClipPath(
          clipper: _NoteFoldClipper(),
          child: Material(
            color: backgroundColor,
            child: IntrinsicHeight(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      ReorderableDragStartListener(
                        index: index,
                        child: Container(
                          width: 32,
                          padding: const Pad(horizontal: 8, vertical: 12),
                          color: Colors.transparent,
                          child: Icon(LucideLightIcons.grip_vertical, color: context.colors.textFaint, size: 16),
                        ),
                      ),
                      Expanded(
                        child: Padding(
                          padding: const Pad(top: 12, right: 12, bottom: 12),
                          child: isExpanded
                              ? TextField(
                                  controller: controller,
                                  focusNode: focusNode,
                                  smartDashesType: SmartDashesType.disabled,
                                  smartQuotesType: SmartQuotesType.disabled,
                                  autocorrect: false,
                                  keyboardType: TextInputType.multiline,
                                  maxLines: null,
                                  minLines: 3,
                                  decoration: const InputDecoration.collapsed(
                                    hintText: '기억할 내용이나 작성에 도움이 되는 내용을 자유롭게 적어보세요.',
                                  ),
                                  onChanged: onUpdateContent,
                                )
                              : GestureDetector(
                                  onTap: onExpand,
                                  child: Text(
                                    controller?.text.replaceAll('\n', ' ') ?? '',
                                    maxLines: 3,
                                    overflow: TextOverflow.ellipsis,
                                  ),
                                ),
                        ),
                      ),
                    ],
                  ),
                  if (footer != null) footer!,
                ],
              ),
            ),
          ),
        ),
        Positioned(
          bottom: 0,
          right: 0,
          child: ClipPath(
            clipper: _NoteTriangleClipper(),
            child: Container(
              width: 12,
              height: 12,
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: [Colors.black.withValues(alpha: 0.05), Colors.black.withValues(alpha: 0.15)],
                ),
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class NoteFooterEntity extends StatelessWidget {
  const NoteFooterEntity({
    super.key,
    required this.entityTitle,
    required this.entityIcon,
    required this.isExpanded,
    required this.onSelectEntity,
    required this.onNavigateToEntity,
  });

  final String entityTitle;
  final IconData entityIcon;
  final bool isExpanded;
  final VoidCallback onSelectEntity;
  final VoidCallback onNavigateToEntity;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: isExpanded ? onSelectEntity : onNavigateToEntity,
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(entityIcon, size: 12, color: context.colors.textSubtle),
          const SizedBox(width: 4),
          Flexible(
            child: Text(
              entityTitle,
              style: TextStyle(fontSize: 12, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              overflow: TextOverflow.ellipsis,
            ),
          ),
          if (isExpanded) ...[
            const SizedBox(width: 4),
            Icon(LucideLightIcons.chevron_down, size: 14, color: context.colors.textSubtle),
          ],
        ],
      ),
    );
  }
}

class NoteFooter extends StatelessWidget {
  const NoteFooter({
    super.key,
    this.entity,
    required this.isExpanded,
    required this.onDelete,
    required this.onCollapse,
    required this.onExpand,
  });

  final NoteFooterEntity? entity;
  final bool isExpanded;
  final VoidCallback onDelete;
  final VoidCallback onCollapse;
  final VoidCallback onExpand;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const Pad(horizontal: 12, bottom: 8),
      child: Row(
        children: [
          if (entity != null) Expanded(child: entity!) else const Spacer(),
          const SizedBox(width: 8),
          if (isExpanded) ...[
            Tappable(
              onTap: onDelete,
              child: Container(
                padding: const Pad(all: 4),
                decoration: BoxDecoration(borderRadius: BorderRadius.circular(4), color: Colors.transparent),
                child: Icon(LucideLightIcons.trash_2, color: context.colors.textDefault, size: 16),
              ),
            ),
            const SizedBox(width: 4),
            Tappable(
              onTap: onCollapse,
              child: Container(
                padding: const Pad(all: 4),
                decoration: BoxDecoration(borderRadius: BorderRadius.circular(4), color: Colors.transparent),
                child: Icon(LucideLightIcons.minimize_2, color: context.colors.textDefault, size: 16),
              ),
            ),
          ] else ...[
            Tappable(
              onTap: onExpand,
              child: Container(
                padding: const Pad(all: 4),
                decoration: BoxDecoration(borderRadius: BorderRadius.circular(4), color: Colors.transparent),
                child: Icon(LucideLightIcons.maximize_2, color: context.colors.textDefault, size: 16),
              ),
            ),
          ],
        ],
      ),
    );
  }
}
