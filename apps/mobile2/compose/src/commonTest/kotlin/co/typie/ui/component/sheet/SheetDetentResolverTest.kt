package co.typie.ui.component.sheet

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class SheetDetentResolverTest {

  @Test
  fun intrinsicLeavesDefaultTopGap() {
    val resolved =
      SheetDetentResolver.resolve(
        policy = SheetSizePolicy.Intrinsic(),
        context = SheetDetentContext(viewportHeight = 600.dp, contentHeight = 720.dp),
      )

    assertEquals(listOf(536.dp), resolved.map { it.height })
    assertEquals(SheetDetentId.Intrinsic, resolved.single().id)
  }

  @Test
  fun intrinsicCanOverrideTopGap() {
    val resolved =
      SheetDetentResolver.resolve(
        policy = SheetSizePolicy.Intrinsic(topGap = 0.dp),
        context = SheetDetentContext(viewportHeight = 600.dp, contentHeight = 720.dp),
      )

    assertEquals(listOf(600.dp), resolved.map { it.height })
    assertEquals(SheetDetentId.Intrinsic, resolved.single().id)
  }

  @Test
  fun maxPolicyLeavesConfiguredTopGap() {
    val resolved =
      SheetDetentResolver.resolve(
        policy = SheetSizePolicy.Max(topGap = 64.dp),
        context = SheetDetentContext(viewportHeight = 800.dp, contentHeight = 200.dp),
      )

    assertEquals(736.dp, resolved.single().height)
    assertEquals(SheetDetentId.TopGap(64.dp), resolved.single().id)
  }

  @Test
  fun maxPolicyUsesDefaultTopGap() {
    val resolved =
      SheetDetentResolver.resolve(
        policy = SheetSizePolicy.Max(),
        context = SheetDetentContext(viewportHeight = 800.dp, contentHeight = 200.dp),
      )

    assertEquals(736.dp, resolved.single().height)
    assertEquals(SheetDetentId.TopGap(64.dp), resolved.single().id)
  }

  @Test
  fun detentPolicyResolvesAndSortsDistinctHeights() {
    val resolved =
      SheetDetentResolver.resolve(
        policy =
          SheetSizePolicy.Detents(
            initial = SheetDetent.Fixed(280.dp),
            available =
              listOf(
                SheetDetent.TopGap(80.dp),
                SheetDetent.Fixed(280.dp),
                SheetDetent.Fraction(0.5f),
              ),
          ),
        context = SheetDetentContext(viewportHeight = 900.dp, contentHeight = 700.dp),
      )

    assertEquals(listOf(280.dp, 450.dp, 820.dp), resolved.map { it.height })
    assertEquals(
      listOf(
        SheetDetentId.Fixed(280.dp),
        SheetDetentId.Fraction(0.5f),
        SheetDetentId.TopGap(80.dp),
      ),
      resolved.map { it.id },
    )
  }

  @Test
  fun fixedAndTopGapDetentsDoNotRequireContentMeasurement() {
    val policy =
      SheetSizePolicy.Detents(
        initial = SheetDetent.Fixed(360.dp),
        available = listOf(SheetDetent.Fixed(360.dp), SheetDetent.TopGap(64.dp)),
      )

    assertEquals(false, policy.requiresContentMeasurement())
  }

  @Test
  fun intrinsicAndContentDetentsRequireContentMeasurement() {
    val policy =
      SheetSizePolicy.Detents(
        initial = SheetDetent.Intrinsic,
        available = listOf(SheetDetent.Content(maxTopGap = 64.dp)),
      )

    assertEquals(true, policy.requiresContentMeasurement())
  }

  @Test
  fun settleDetentPrefersExpansionDirectionAfterUpwardDrag() {
    val detents =
      listOf(
        ResolvedSheetDetent(SheetDetentId.Fixed(360.dp), 360.dp),
        ResolvedSheetDetent(SheetDetentId.TopGap(64.dp), 736.dp),
      )

    val settled =
      resolveSheetSettledDetent(
        policy =
          SheetSizePolicy.Detents(
            initial = SheetDetent.Fixed(360.dp),
            available = listOf(SheetDetent.Fixed(360.dp), SheetDetent.TopGap(64.dp)),
          ),
        detents = detents,
        currentDetentId = SheetDetentId.Fixed(360.dp),
        sheetHeight = 408.dp,
        velocity = 0f,
      )

    assertEquals(SheetDetentId.TopGap(64.dp), settled?.id)
  }

  @Test
  fun settleDetentHonorsProgrammaticOnlyExpansionPolicy() {
    val detents =
      listOf(
        ResolvedSheetDetent(SheetDetentId.Fixed(360.dp), 360.dp),
        ResolvedSheetDetent(SheetDetentId.TopGap(64.dp), 736.dp),
      )

    val settled =
      resolveSheetSettledDetent(
        policy =
          SheetSizePolicy.Detents(
            initial = SheetDetent.Fixed(360.dp),
            available = listOf(SheetDetent.Fixed(360.dp), SheetDetent.TopGap(64.dp)),
            expansionPolicy = SheetExpansionPolicy.ProgrammaticOnly,
          ),
        detents = detents,
        currentDetentId = SheetDetentId.Fixed(360.dp),
        sheetHeight = 440.dp,
        velocity = -480f,
      )

    assertEquals(SheetDetentId.Fixed(360.dp), settled?.id)
  }

  @Test
  fun fromCurrentDetentSkipsIntermediateCollapseDetentsDuringDragDismiss() {
    val detents =
      listOf(
        ResolvedSheetDetent(SheetDetentId.Fixed(360.dp), 360.dp),
        ResolvedSheetDetent(SheetDetentId.TopGap(128.dp), 672.dp),
      )

    val settled =
      resolveSheetSettledDetent(
        policy =
          SheetSizePolicy.Detents(
            initial = SheetDetent.Fixed(360.dp),
            available = listOf(SheetDetent.Fixed(360.dp), SheetDetent.TopGap(128.dp)),
            dragDismissBehavior = SheetDragDismissBehavior.FromCurrentDetent,
          ),
        detents = detents,
        currentDetentId = SheetDetentId.TopGap(128.dp),
        sheetHeight = 500.dp,
        velocity = 0f,
      )

    assertEquals(SheetDetentId.TopGap(128.dp), settled?.id)
  }

  @Test
  fun dragDismissDefaultsToMinDetentAnchor() {
    val detents =
      listOf(
        ResolvedSheetDetent(SheetDetentId.Fixed(360.dp), 360.dp),
        ResolvedSheetDetent(SheetDetentId.TopGap(128.dp), 672.dp),
      )

    val shouldDismiss =
      shouldDismissDraggedSheet(
        policy =
          SheetSizePolicy.Detents(
            initial = SheetDetent.Fixed(360.dp),
            available = listOf(SheetDetent.Fixed(360.dp), SheetDetent.TopGap(128.dp)),
          ),
        detents = detents,
        currentDetentId = SheetDetentId.TopGap(128.dp),
        sheetHeight = 430.dp,
        velocity = 0f,
      )

    assertEquals(false, shouldDismiss)
  }

  @Test
  fun dragDismissCanUseCurrentDetentAnchorWithoutSnappingToMin() {
    val detents =
      listOf(
        ResolvedSheetDetent(SheetDetentId.Fixed(360.dp), 360.dp),
        ResolvedSheetDetent(SheetDetentId.TopGap(128.dp), 672.dp),
      )

    val shouldDismiss =
      shouldDismissDraggedSheet(
        policy =
          SheetSizePolicy.Detents(
            initial = SheetDetent.Fixed(360.dp),
            available = listOf(SheetDetent.Fixed(360.dp), SheetDetent.TopGap(128.dp)),
            dragDismissBehavior = SheetDragDismissBehavior.FromCurrentDetent,
          ),
        detents = detents,
        currentDetentId = SheetDetentId.TopGap(128.dp),
        sheetHeight = 430.dp,
        velocity = 0f,
      )

    assertEquals(true, shouldDismiss)
  }
}
