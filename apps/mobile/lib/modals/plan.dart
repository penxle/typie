import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:in_app_purchase/in_app_purchase.dart';
import 'package:in_app_purchase_android/in_app_purchase_android.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/widgets/btn.dart';

enum BillingCycle { monthly, yearly }

class PlanModal extends HookWidget {
  const PlanModal({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final authState = useValueListenable(auth);

    final future = useMemoized(_fetchProductMap);
    final productDetailsMap = useFuture(future);

    return Column(
      mainAxisSize: MainAxisSize.min,
      spacing: 8,
      children: [
        Btn(
          '월 결제 (${productDetailsMap.data?[BillingCycle.monthly]!.price})',
          onTap: () async {
            await _purchaseProduct(authState, productDetailsMap.data![BillingCycle.monthly]!);
          },
        ),
        Btn(
          '연 결제 (${productDetailsMap.data?[BillingCycle.yearly]!.price})',
          onTap: () async {
            await _purchaseProduct(authState, productDetailsMap.data![BillingCycle.yearly]!);
          },
        ),
        Btn(
          'Restore purchases',
          onTap: () async {
            await _restorePurchases(authState);
          },
        ),
      ],
    );
  }
}

class _Product {
  _Product(this.details) {
    id = details.id;
    price = '${details.currencySymbol}${details.rawPrice.comma}';
  }

  late final String id;
  late final String price;
  final ProductDetails details;
}

Future<Map<BillingCycle, _Product>> _fetchProductMap() async {
  if (Platform.isIOS) {
    final response = await InAppPurchase.instance.queryProductDetails({'plan.full.1month', 'plan.full.1year'});
    return {
      BillingCycle.monthly: _Product(
        response.productDetails.firstWhere((productDetails) => productDetails.id == 'plan.full.1month'),
      ),
      BillingCycle.yearly: _Product(
        response.productDetails.firstWhere((productDetails) => productDetails.id == 'plan.full.1year'),
      ),
    };
  } else {
    final response = await InAppPurchase.instance.queryProductDetails({'plan.full'});
    return {
      BillingCycle.monthly: _Product(
        response.productDetails.firstWhere(
          (productDetails) =>
              productDetails is GooglePlayProductDetails &&
              productDetails.productDetails.subscriptionOfferDetails![productDetails.subscriptionIndex!].basePlanId ==
                  '1month',
        ),
      ),
      BillingCycle.yearly: _Product(
        response.productDetails.firstWhere(
          (productDetails) =>
              productDetails is GooglePlayProductDetails &&
              productDetails.productDetails.subscriptionOfferDetails![productDetails.subscriptionIndex!].basePlanId ==
                  '1year',
        ),
      ),
    };
  }
}

Future<void> _purchaseProduct(AuthState authState, _Product product) async {
  if (authState case Authenticated(:final me)) {
    if (Platform.isIOS) {
      await InAppPurchase.instance.buyNonConsumable(
        purchaseParam: PurchaseParam(productDetails: product.details, applicationUserName: me.uuid),
      );
    } else {
      await InAppPurchase.instance.buyNonConsumable(
        purchaseParam: GooglePlayPurchaseParam(productDetails: product.details, applicationUserName: me.uuid),
      );
    }
  }
}

Future<void> _restorePurchases(AuthState authState) async {
  if (authState case Authenticated(:final me)) {
    await InAppPurchase.instance.restorePurchases(applicationUserName: me.uuid);
  }
}
