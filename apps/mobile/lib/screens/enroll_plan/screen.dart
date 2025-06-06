import 'dart:io';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:in_app_purchase/in_app_purchase.dart';
import 'package:in_app_purchase_android/in_app_purchase_android.dart';
import 'package:sentry_flutter/sentry_flutter.dart';
import 'package:typie/context/loader.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/logger.dart';
import 'package:typie/screens/enroll_plan/__generated__/screen.req.gql.dart';
import 'package:typie/screens/profile/__generated__/screen.req.gql.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

enum PlanInterval { monthly, yearly }

@RoutePage()
class EnrollPlanScreen extends HookWidget {
  const EnrollPlanScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final future = useMemoized(_fetchProductMap);
    final productDetailsMap = useFuture(future);

    useEffect(() {
      final subscription = InAppPurchase.instance.purchaseStream.listen((purchaseDetailsList) async {
        for (final purchaseDetails in purchaseDetailsList) {
          try {
            if (purchaseDetails.status == PurchaseStatus.purchased ||
                purchaseDetails.status == PurchaseStatus.restored) {
              await client.request(
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

              if (context.mounted) {
                await context.showModal(
                  child: const AlertModal(title: '구독이 완료되었어요', message: '타이피의 모든 기능을 이용해보세요!'),
                );
              }
            }
          } catch (err) {
            log.e('EnrollPlanScreen', error: err);
            await Sentry.captureException(err);
          } finally {
            if (purchaseDetails.pendingCompletePurchase) {
              await InAppPurchase.instance.completePurchase(purchaseDetails);
            }
          }
        }
      });

      return subscription.cancel;
    }, []);

    return Screen(
      heading: const Heading(title: '이용권 구매/변경'),
      padding: const Pad(all: 20),
      child: GraphQLOperation(
        operation: GEnrollPlanScreen_QueryReq(),
        builder: (context, client, data) {
          return Column(
            spacing: 12,
            children: [
              if (data.me!.subscription == null)
                Container(
                  decoration: BoxDecoration(
                    border: Border.all(color: AppColors.gray_950),
                    borderRadius: BorderRadius.circular(8),
                    color: AppColors.white,
                  ),
                  padding: const Pad(all: 16),
                  child: const Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Row(
                        children: [
                          Text('타이피 BASIC ACCESS', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
                          Spacer(),
                          Text('현재 이용중', style: TextStyle(fontSize: 14, color: AppColors.gray_700)),
                        ],
                      ),
                      Gap(12),
                      HorizontalDivider(color: AppColors.gray_950),
                      Gap(12),
                      Column(
                        spacing: 8,
                        children: [
                          _FeatureItem(icon: LucideLightIcons.book_open_text, label: '16,000자까지 작성 가능'),
                          _FeatureItem(icon: LucideLightIcons.images, label: '20MB까지 파일 업로드 가능'),
                        ],
                      ),
                    ],
                  ),
                ),
              Container(
                decoration: BoxDecoration(
                  border: Border.all(color: AppColors.gray_950),
                  borderRadius: BorderRadius.circular(8),
                  color: AppColors.white,
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    const Padding(
                      padding: Pad(all: 16),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          Text('타이피 FULL ACCESS', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
                          Gap(12),
                          HorizontalDivider(color: AppColors.gray_950),
                          Gap(12),
                          Column(
                            spacing: 8,
                            children: [
                              _FeatureItem(icon: LucideLightIcons.book_open_text, label: '무제한 글자 수'),
                              _FeatureItem(icon: LucideLightIcons.images, label: '무제한 파일 업로드'),
                              _FeatureItem(icon: LucideLightIcons.link, label: '커스텀 공유 주소'),
                              _FeatureItem(icon: LucideLightIcons.flask_conical, label: '베타 기능 우선 접근'),
                              _FeatureItem(icon: LucideLightIcons.headset, label: '문제 발생 시 우선 지원'),
                              _FeatureItem(icon: LucideLightIcons.sprout, label: '디스코드 커뮤니티 참여'),
                              _FeatureItem(icon: LucideLightIcons.ellipsis, label: '그리고 더 많은 혜택'),
                            ],
                          ),
                        ],
                      ),
                    ),
                    const HorizontalDivider(color: AppColors.gray_950),
                    Padding(
                      padding: const Pad(all: 16),
                      child: Column(
                        spacing: 12,
                        children: [
                          _PurchaseButton(
                            label: '1개월 구독하기',
                            product: productDetailsMap.data?[PlanInterval.monthly],
                            onTap: (product) async {
                              await context.runWithLoader(() async {
                                await _purchaseProduct(product, uuid: data.me!.uuid);
                              });
                            },
                          ),
                          _PurchaseButton(
                            label: '1년 구독하기',
                            product: productDetailsMap.data?[PlanInterval.yearly],
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
          );
        },
      ),
    );
  }
}

class _FeatureItem extends StatelessWidget {
  const _FeatureItem({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Row(
      spacing: 8,
      children: [
        Icon(icon, size: 16),
        Text(label, style: const TextStyle(fontSize: 14)),
      ],
    );
  }
}

class _PurchaseButton extends StatelessWidget {
  const _PurchaseButton({required this.label, required this.onTap, this.product});

  final _Product? product;
  final String label;
  final void Function(_Product product) onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: () {
        if (product == null) {
          return;
        }

        onTap(product!);
      },
      child: Container(
        decoration: BoxDecoration(
          border: Border.all(color: AppColors.gray_950),
          borderRadius: BorderRadius.circular(8),
        ),
        padding: const Pad(all: 12),
        child: Row(
          children: [
            Text(label, style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600)),
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
