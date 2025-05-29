import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:pointer_interceptor/pointer_interceptor.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';

extension ModalExtension on BuildContext {
  Future<T?> showModal<T extends Object?>({required Widget child, bool intercept = false}) {
    return router.root.pushWidget(
      child,
      opaque: false,
      transitionDuration: const Duration(milliseconds: 200),
      transitionBuilder: (context, animation, secondaryAnimation, child) {
        final tweenedBackdropOpacity = Tween<double>(
          begin: 0,
          end: 1,
        ).animate(CurvedAnimation(parent: animation, curve: Curves.easeOutCubic, reverseCurve: Curves.easeIn));

        final tweenedOpacity = Tween<double>(
          begin: 0,
          end: 1,
        ).animate(CurvedAnimation(parent: animation, curve: Curves.easeOutCubic, reverseCurve: Curves.easeIn));

        return Stack(
          children: [
            Positioned.fill(
              child: PointerInterceptor(
                intercepting: intercept,
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  child: FadeTransition(
                    opacity: tweenedBackdropOpacity,
                    child: SizedBox.expand(child: ColoredBox(color: AppColors.black.withValues(alpha: 0.5))),
                  ),
                  onTap: () async {
                    await router.root.maybePop();
                  },
                ),
              ),
            ),
            Padding(
              padding: const Pad(horizontal: 40),
              child: Center(
                child: FadeTransition(opacity: tweenedOpacity, child: child),
              ),
            ),
          ],
        );
      },
    );
  }
}

class Modal extends StatelessWidget {
  const Modal({required this.child, this.floating = false, super.key});

  final Widget child;
  final bool floating;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: AppColors.transparent,
      child: Container(
        width: 300,
        decoration: BoxDecoration(
          color: AppColors.white,
          border: Border.all(color: AppColors.gray_950),
          borderRadius: BorderRadius.circular(16),
          boxShadow: [
            BoxShadow(offset: const Offset(0, 1), blurRadius: 2, color: AppColors.gray_950.withValues(alpha: 0.07)),
            BoxShadow(offset: const Offset(0, 2), blurRadius: 4, color: AppColors.gray_950.withValues(alpha: 0.07)),
            BoxShadow(offset: const Offset(0, 4), blurRadius: 8, color: AppColors.gray_950.withValues(alpha: 0.07)),
            BoxShadow(offset: const Offset(0, 8), blurRadius: 16, color: AppColors.gray_950.withValues(alpha: 0.07)),
          ],
        ),
        child: Padding(padding: const Pad(all: 20), child: child),
      ),
    );
  }
}

class AlertModal extends StatelessWidget {
  const AlertModal({
    required this.title,
    required this.message,
    this.onConfirm,
    this.confirmText = '확인',
    this.confirmColor = AppColors.gray_950,
    super.key,
  });

  final String title;
  final String message;

  final String confirmText;
  final Color? confirmColor;
  final void Function()? onConfirm;

  @override
  Widget build(BuildContext context) {
    return Modal(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(title, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
          const Gap(8),
          Text(message, style: const TextStyle(fontSize: 16)),
          const Gap(24),
          Tappable(
            onTap: () async {
              await context.router.maybePop();
              onConfirm?.call();
            },
            child: Container(
              alignment: Alignment.center,
              decoration: BoxDecoration(color: confirmColor, borderRadius: BorderRadius.circular(999)),
              padding: const Pad(vertical: 12),
              child: Text(
                confirmText,
                style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600, color: AppColors.white),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class ConfirmModal extends StatelessWidget {
  const ConfirmModal({
    required this.title,
    required this.message,
    required this.onConfirm,
    this.onCancel,
    this.confirmText = '확인',
    this.cancelText = '취소',
    this.confirmColor = AppColors.gray_950,
    this.cancelColor = AppColors.gray_100,
    super.key,
  });

  final String title;
  final String message;

  final String confirmText;
  final String cancelText;

  final Color? confirmColor;
  final Color? cancelColor;

  final void Function() onConfirm;
  final void Function()? onCancel;

  @override
  Widget build(BuildContext context) {
    return Modal(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(title, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
          const Gap(8),
          Text(message, style: const TextStyle(fontSize: 16)),
          const Gap(24),
          Row(
            spacing: 8,
            children: [
              Expanded(
                child: Tappable(
                  onTap: () async {
                    await context.router.maybePop();
                    onCancel?.call();
                  },
                  child: Container(
                    alignment: Alignment.center,
                    decoration: BoxDecoration(color: cancelColor, borderRadius: BorderRadius.circular(999)),
                    padding: const Pad(vertical: 12),
                    child: Text(cancelText, style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
                  ),
                ),
              ),
              Expanded(
                child: Tappable(
                  onTap: () async {
                    await context.router.maybePop();
                    onConfirm();
                  },
                  child: Container(
                    alignment: Alignment.center,
                    decoration: BoxDecoration(color: confirmColor, borderRadius: BorderRadius.circular(999)),
                    padding: const Pad(vertical: 12),
                    child: Text(
                      confirmText,
                      style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600, color: AppColors.white),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
