import 'dart:async';
import 'dart:io';

import 'package:appsflyer_sdk/appsflyer_sdk.dart';
import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:in_app_purchase/in_app_purchase.dart';
import 'package:in_app_purchase_android/in_app_purchase_android.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:sentry_flutter/sentry_flutter.dart';
import 'package:typie/constants/plan_features.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/loader.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/logger.dart';
import 'package:typie/screens/enroll_plan/__generated__/screen_query.data.gql.dart';
import 'package:typie/screens/enroll_plan/__generated__/screen_query.req.gql.dart';
import 'package:typie/screens/enroll_plan/__generated__/subscribe_or_change_plan_with_in_app_purchase_mutation.req.gql.dart';
import 'package:typie/screens/enroll_plan/__generated__/subscribe_plan_with_trial_mutation.req.gql.dart';
import 'package:typie/screens/enroll_plan/subscription_celebration_bottom_sheet.dart';
import 'package:typie/screens/profile/__generated__/profile_query.req.gql.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/overlay_heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

enum PlanInterval { monthly, yearly }

const _cardRadius = 12.0;
const _sectionGap = 16.0;

@RoutePage()
class EnrollPlanScreen extends HookWidget {
  const EnrollPlanScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final mixpanel = useService<Mixpanel>();
    final appsflyer = useService<AppsflyerSdk>();

    final future = useMemoized(_fetchProductMap);
    final productDetailsMap = useFuture(future);

    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceSubtle,
      operation: GEnrollPlanScreen_QueryReq(),
      builder: (context, client, data) => _Content(
        data: data,
        client: client,
        mixpanel: mixpanel,
        appsflyer: appsflyer,
        productDetailsMap: productDetailsMap.data,
      ),
    );
  }
}

class _Content extends HookWidget {
  const _Content({
    required this.data,
    required this.client,
    required this.mixpanel,
    required this.appsflyer,
    required this.productDetailsMap,
  });

  final GEnrollPlanScreen_QueryData data;
  final GraphQLClient client;
  final Mixpanel mixpanel;
  final AppsflyerSdk appsflyer;
  final Map<PlanInterval, _Product>? productDetailsMap;

  @override
  Widget build(BuildContext context) {
    final scrollController = useScrollController();

    useEffect(() {
      final originalSubscriptionId = data.me!.subscription?.id;
      final originalPlanId = data.me!.subscription?.plan.id;

      final subscription = InAppPurchase.instance.purchaseStream.listen((purchaseDetailsList) async {
        for (final purchaseDetails in purchaseDetailsList) {
          try {
            if (purchaseDetails.status == PurchaseStatus.purchased ||
                purchaseDetails.status == PurchaseStatus.restored) {
              final resp = await client.request(
                GEnrollPlanScreen_SubscribeOrChangePlanWithInAppPurchase_MutationReq(
                  (b) => b
                    ..vars.input.store = Platform.isIOS
                        ? GInAppPurchaseStore.APP_STORE
                        : GInAppPurchaseStore.GOOGLE_PLAY
                    ..vars.input.data = Platform.isIOS
                        ? purchaseDetails.purchaseID
                        : purchaseDetails.verificationData.serverVerificationData,
                ),
              );

              await client.refetch(GEnrollPlanScreen_QueryReq());
              await client.refetch(GProfileScreen_QueryReq());

              if (resp.subscribeOrChangePlanWithInAppPurchase.id == originalSubscriptionId &&
                  resp.subscribeOrChangePlanWithInAppPurchase.plan.id == originalPlanId) {
                return;
              }

              final productDetails = productDetailsMap?.entries
                  .firstWhereOrNull((e) => e.value.details.id == purchaseDetails.productID)
                  ?.value
                  .details;

              unawaited(mixpanel.track('enroll_plan', properties: {'productId': purchaseDetails.productID}));
              unawaited(
                appsflyer.logEvent('complete_subscription', {
                  'product_id': productDetails?.id,
                  'product_name': productDetails?.title,
                  'price': productDetails?.rawPrice,
                  'currency': productDetails?.currencyCode,
                }),
              );

              if (context.mounted) {
                await context.showBottomSheet(
                  child: const SubscriptionCelebrationBottomSheet(
                    title: '구독이 시작됐어요!',
                    message: '타이피의 모든 기능을 자유롭게 이용해보세요.',
                  ),
                );
              }
            }
          } catch (err) {
            await Sentry.captureException(err);
            log.e('EnrollPlanScreen', error: err);
          } finally {
            if (purchaseDetails.pendingCompletePurchase) {
              await InAppPurchase.instance.completePurchase(purchaseDetails);
            }
          }
        }
      });

      return subscription.cancel;
    }, []);

    final isOnTrial = data.me!.subscription?.plan.availability == GPlanAvailability.TRIAL;
    final canStartTrial = data.me!.canStartTrial;
    final bottomPadding = MediaQuery.paddingOf(context).bottom + 72;

    return Screen(
      extendBodyBehindAppBar: true,
      heading: _Heading(scrollController: scrollController),
      child: OverlayHeadingLayout(
        child: SingleChildScrollView(
          controller: scrollController,
          physics: const AlwaysScrollableScrollPhysics(),
          padding: EdgeInsets.fromLTRB(20, 0, 20, bottomPadding),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Padding(
                padding: EdgeInsets.only(top: OverlayHeading.titleTopPadding(context), bottom: 4),
                child: const Text('이용권 구매/변경', style: TextStyle(fontSize: 24, fontWeight: FontWeight.w800)),
              ),
              const Gap(_sectionGap),
              if (data.me!.subscription == null)
                DecoratedBox(
                  decoration: _cardDecoration(context),
                  child: Padding(
                    padding: const Pad(all: 18),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        Row(
                          children: [
                            const Text('타이피 BASIC ACCESS', style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
                            const Spacer(),
                            Text('현재 이용중', style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
                          ],
                        ),
                        const Gap(12),
                        HorizontalDivider(color: context.colors.borderSubtle),
                        const Gap(12),
                        Column(
                          spacing: 8,
                          children: basicPlanFeatures
                              .map((feature) => _FeatureItem(icon: feature.icon, label: feature.label))
                              .toList(),
                        ),
                      ],
                    ),
                  ),
                ),
              if (data.me!.subscription == null) const Gap(_sectionGap),
              DecoratedBox(
                decoration: _cardDecoration(context),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Padding(
                      padding: const Pad(all: 18),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          Row(
                            children: [
                              const Text(
                                '타이피 FULL ACCESS',
                                style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700),
                              ),
                              if (isOnTrial) ...[
                                const Gap(8),
                                Container(
                                  padding: const Pad(horizontal: 8, vertical: 4),
                                  decoration: BoxDecoration(
                                    color: context.colors.accentBrand.withValues(alpha: 0.1),
                                    borderRadius: BorderRadius.circular(6),
                                  ),
                                  child: Text(
                                    '무료 체험 중',
                                    style: TextStyle(
                                      fontSize: 12,
                                      fontWeight: FontWeight.w700,
                                      color: context.colors.accentBrand,
                                    ),
                                  ),
                                ),
                              ],
                            ],
                          ),
                          const Gap(12),
                          HorizontalDivider(color: context.colors.borderSubtle),
                          const Gap(12),
                          Column(
                            spacing: 8,
                            children: fullPlanFeatures
                                .map((feature) => _FeatureItem(icon: feature.icon, label: feature.label))
                                .toList(),
                          ),
                        ],
                      ),
                    ),
                    HorizontalDivider(color: context.colors.borderSubtle),
                    Padding(
                      padding: const Pad(all: 16),
                      child: Column(
                        spacing: 12,
                        children: [
                          if (canStartTrial)
                            _TrialButton(
                              onTap: () async {
                                await context.showBottomSheet(
                                  child: ConfirmBottomSheet(
                                    title: '무료 체험을 시작하시겠어요?',
                                    message: '결제 수단 등록 없이 2주간 타이피의 모든 기능을 무료로 이용할 수 있어요. 체험 종료 후 자동 결제되지 않아요.',
                                    confirmText: '시작하기',
                                    onConfirm: () async {
                                      await context.runWithLoader(() async {
                                        await client.request(GEnrollPlanScreen_SubscribePlanWithTrial_MutationReq());
                                        await client.refetch(GEnrollPlanScreen_QueryReq());
                                        await client.refetch(GProfileScreen_QueryReq());
                                        unawaited(mixpanel.track('start_trial'));
                                      });

                                      if (context.mounted) {
                                        await context.showBottomSheet(
                                          child: const SubscriptionCelebrationBottomSheet(
                                            title: '무료 체험이 시작됐어요!',
                                            message: '2주간 타이피의 모든 기능을 자유롭게 이용해보세요.',
                                          ),
                                        );
                                      }
                                    },
                                  ),
                                );
                              },
                            ),
                          _PurchaseButton(
                            label: '1개월 구독하기',
                            product: productDetailsMap?[PlanInterval.monthly],
                            isActive: data.me!.subscription?.plan.id == 'PL0FL1MAP',
                            onTap: (product) async {
                              await context.runWithLoader(() async {
                                await _purchaseProduct(product, uuid: data.me!.uuid);
                              });
                            },
                          ),
                          _PurchaseButton(
                            label: '1년 구독하기',
                            product: productDetailsMap?[PlanInterval.yearly],
                            isActive: data.me!.subscription?.plan.id == 'PL0FL1YAP',
                            onTap: (product) async {
                              await context.runWithLoader(() async {
                                await _purchaseProduct(product, uuid: data.me!.uuid);
                              });
                            },
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _Heading extends StatelessWidget implements PreferredSizeWidget {
  const _Heading({required this.scrollController});

  final ScrollController scrollController;

  @override
  Widget build(BuildContext context) {
    return OverlayHeading(
      title: '이용권 구매/변경',
      scrollController: scrollController,
      leading: OverlayHeadingBackButton(
        onTap: () async {
          await context.router.maybePop();
        },
      ),
    );
  }

  @override
  Size get preferredSize => const Size.fromHeight(OverlayHeading.height);
}

class _FeatureItem extends StatelessWidget {
  const _FeatureItem({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Row(
      spacing: 10,
      children: [
        Icon(icon, size: 18, color: context.colors.textSubtle),
        Expanded(
          child: Text(label, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500)),
        ),
      ],
    );
  }
}

class _PurchaseButton extends HookWidget {
  const _PurchaseButton({required this.label, required this.onTap, required this.isActive, this.product});

  final _Product? product;
  final String label;
  final bool isActive;
  final void Function(_Product product) onTap;

  @override
  Widget build(BuildContext context) {
    final mixpanel = useService<Mixpanel>();
    final appsflyer = useService<AppsflyerSdk>();

    return Tappable(
      onTap: () {
        if (product == null) {
          return;
        }

        unawaited(mixpanel.track('enroll_plan_try', properties: {'productId': product!.details.id}));
        unawaited(
          appsflyer.logEvent('initiate_subscription', {
            'product_id': product!.details.id,
            'product_name': product!.details.title,
            'price': product!.details.rawPrice,
            'currency': product!.details.currencyCode,
          }),
        );

        onTap(product!);
      },
      child: Container(
        decoration: BoxDecoration(color: context.colors.surfaceSubtle, borderRadius: BorderRadius.circular(10)),
        padding: const Pad(all: 13),
        child: Row(
          children: [
            Text(label, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600)),
            if (isActive) ...[
              const Gap(4),
              Text('(현재 이용중)', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
            ],
            const Spacer(),
            if (product == null)
              const Center(child: SizedBox.square(dimension: 14, child: CircularProgressIndicator()))
            else ...[
              Text(product!.price, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600)),
              const Gap(4),
              const Icon(LucideLightIcons.chevron_right, size: 16),
            ],
          ],
        ),
      ),
    );
  }
}

class _TrialButton extends StatelessWidget {
  const _TrialButton({required this.onTap});

  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      child: Container(
        decoration: BoxDecoration(color: context.colors.surfaceInverse, borderRadius: BorderRadius.circular(10)),
        padding: const Pad(all: 13),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          spacing: 6,
          children: [
            Icon(LucideLightIcons.zap, size: 16, color: context.colors.textInverse),
            Text(
              '2주 무료 체험하기',
              style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: context.colors.textInverse),
            ),
          ],
        ),
      ),
    );
  }
}

BoxDecoration _cardDecoration(BuildContext context) =>
    BoxDecoration(color: context.colors.surfaceDefault, borderRadius: BorderRadius.circular(_cardRadius));

class _Product {
  const _Product(this.details);

  final ProductDetails details;

  String get price => details.price;
}

Future<Map<PlanInterval, _Product>> _fetchProductMap() async {
  if (Platform.isIOS) {
    final response = await InAppPurchase.instance.queryProductDetails({'pl0fl1map', 'pl0fl1yap'});
    return {
      PlanInterval.monthly: _Product(
        response.productDetails.firstWhere((productDetails) => productDetails.id == 'pl0fl1map'),
      ),
      PlanInterval.yearly: _Product(
        response.productDetails.firstWhere((productDetails) => productDetails.id == 'pl0fl1yap'),
      ),
    };
  } else {
    final response = await InAppPurchase.instance.queryProductDetails({'plan.full'});
    return {
      PlanInterval.monthly: _Product(
        response.productDetails.firstWhere(
          (productDetails) =>
              productDetails is GooglePlayProductDetails &&
              productDetails.productDetails.subscriptionOfferDetails![productDetails.subscriptionIndex!].basePlanId ==
                  'pl0fl1map',
        ),
      ),
      PlanInterval.yearly: _Product(
        response.productDetails.firstWhere(
          (productDetails) =>
              productDetails is GooglePlayProductDetails &&
              productDetails.productDetails.subscriptionOfferDetails![productDetails.subscriptionIndex!].basePlanId ==
                  'pl0fl1yap',
        ),
      ),
    };
  }
}

var _isPurchasing = false;
Future<void> _purchaseProduct(_Product product, {required String uuid}) async {
  if (_isPurchasing) {
    return;
  }

  try {
    _isPurchasing = true;

    await InAppPurchase.instance.buyNonConsumable(
      purchaseParam: PurchaseParam(productDetails: product.details, applicationUserName: uuid),
    );
  } catch (_) {
    // pass
  } finally {
    _isPurchasing = false;
  }
}
