import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/widgets.dart';
import 'package:fluttertoast/fluttertoast.dart';
import 'package:typie/icons/lucide.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';

class _TextToastWidget extends StatelessWidget {
  const _TextToastWidget({required this.type, required this.message, this.actionText, this.onAction});

  final ToastType type;
  final String message;
  final String? actionText;
  final void Function()? onAction;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const Pad(all: 14),
      decoration: BoxDecoration(color: AppColors.gray_600, borderRadius: BorderRadius.circular(4)),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Padding(
            padding: const Pad(top: 2),
            child: switch (type) {
              ToastType.success => const Icon(LucideIcons.circle_check, color: AppColors.green_500, size: 16),
              ToastType.error => const Icon(LucideIcons.circle_alert, color: AppColors.red_500, size: 16),
            },
          ),
          const Box.gap(6),
          Expanded(
            child: Text(
              message,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: AppColors.white),
            ),
          ),
          if (actionText != null) ...[
            const Box.gap(16),
            Tappable(
              onTap: onAction,
              child: Text(
                actionText!,
                style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w700, color: AppColors.white),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class ToastProvider extends StatelessWidget {
  const ToastProvider({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Overlay(
      initialEntries: <OverlayEntry>[
        OverlayEntry(
          builder: (context) {
            return ToastScope(toast: FToast()..init(context), child: child);
          },
        ),
      ],
    );
  }
}

class ToastScope extends InheritedWidget {
  const ToastScope({required this.toast, required super.child, super.key});

  final FToast toast;

  static FToast of(BuildContext context) {
    final scope = context.getInheritedWidgetOfExactType<ToastScope>();
    return scope!.toast;
  }

  @override
  bool updateShouldNotify(ToastScope old) => false;
}

enum ToastType { success, error }

class ToastExtension {
  const ToastExtension(this.context);

  final BuildContext context;
  FToast get toast => ToastScope.of(context);

  void show(ToastType type, String message, {String? actionText, void Function()? onAction}) {
    toast.showToast(
      child: _TextToastWidget(
        type: type,
        message: message,
        actionText: actionText,
        onAction: () {
          toast.removeCustomToast();
          onAction?.call();
        },
      ),
      positionedToastBuilder: (context, child, gravity) {
        final height = MediaQuery.of(context).padding.bottom;
        final inset = MediaQuery.of(context).viewInsets.bottom;
        return Positioned(bottom: height + inset + 12, left: 20, right: 20, child: child);
      },
    );
  }
}

extension ToastX on BuildContext {
  ToastExtension get toast => ToastExtension(this);
}
