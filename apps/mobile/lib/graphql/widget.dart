import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:ferry/ferry.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:sentry_flutter/sentry_flutter.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/logger.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';

class GraphQLOperation<TData, TVars> extends HookWidget {
  const GraphQLOperation({
    required this.operation,
    required this.builder,
    this.initialBackgroundColor,
    this.onLoaded,
    super.key,
  });

  final OperationRequest<TData, TVars> operation;
  final Widget Function(BuildContext context, GraphQLClient client, TData data) builder;
  final Color? initialBackgroundColor;
  final void Function(TData data)? onLoaded;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final stream = useMemoized(() => client.raw.request(operation).distinct(), [operation]);
    final snapshot = useStream(stream);
    final loaded = useRef(false);

    final controller = useAnimationController(duration: const Duration(milliseconds: 200));
    final tweenedOpacity = useMemoized(() {
      final curve = CurvedAnimation(parent: controller, curve: Curves.ease);
      return Tween<double>(begin: 0, end: 1).animate(curve);
    }, [controller]);

    useEffect(() {
      final data = snapshot.data?.data;
      if (data != null && !loaded.value) {
        loaded.value = true;
        controller.forward();
        onLoaded?.call(data);
      }

      return null;
    }, [snapshot.data]);

    final error = snapshot.error ?? snapshot.data?.graphqlErrors ?? snapshot.data?.linkException;
    if (error != null) {
      unawaited(Sentry.captureException(error));
      log.e(error);

      return Material(
        color: initialBackgroundColor ?? AppColors.transparent,
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Text('앗! 문제가 발생했어요', style: TextStyle(fontSize: 16)),
            const Text('잠시 후 다시 시도해주세요.', style: TextStyle(fontSize: 15, color: AppColors.gray_500)),
            const Gap(16),
            Tappable(
              onTap: () async {
                await client.refetch(operation);
              },
              child: Container(
                decoration: BoxDecoration(
                  border: Border.all(color: AppColors.gray_950),
                  borderRadius: BorderRadius.circular(8),
                ),
                padding: const Pad(horizontal: 16, vertical: 8),
                child: const Text('다시 시도하기', style: TextStyle(fontSize: 15)),
              ),
            ),
          ],
        ),
      );
    }

    final data = snapshot.data?.data;
    final child = data == null
        ? const SizedBox.shrink()
        : FadeTransition(opacity: tweenedOpacity, child: builder(context, client, data));

    if (initialBackgroundColor == null) {
      return child;
    } else {
      return ColoredBox(color: initialBackgroundColor!, child: child);
    }
  }
}
