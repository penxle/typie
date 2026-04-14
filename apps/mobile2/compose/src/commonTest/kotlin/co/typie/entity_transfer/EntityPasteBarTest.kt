package co.typie.entity_transfer

import androidx.compose.ui.unit.dp
import co.typie.domain.entity_transfer.entityPasteBarToastBottomInset
import kotlin.test.Test
import kotlin.test.assertEquals

class EntityPasteBarTest {
  @Test
  fun `paste bar toast inset adds only paste bar height`() {
    assertEquals(120.dp, entityPasteBarToastBottomInset(72.dp))
  }
}
