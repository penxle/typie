package co.typie.domain.entity

import androidx.compose.ui.text.AnnotatedString
import co.typie.domain.entitytransfer.EntityTransferSource
import co.typie.domain.entitytransfer.toTransferSource
import co.typie.graphql.FolderScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.SearchScreen_Search_Query
import co.typie.graphql.SpaceScreen_Query
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildDocument
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.builder.buildFolder
import co.typie.graphql.builder.buildSearchHitDocument
import co.typie.graphql.builder.buildSearchResult
import co.typie.graphql.builder.buildSite
import co.typie.graphql.fragment.EntityDetails_entity
import co.typie.graphql.fragment.EntityRow_entity
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class EntityRowTest {
  @Test
  fun `document breadcrumb names prepend site and ancestors`() {
    val entity = detailsEntity {
      id = "entity-1"
      slug = "hello-world"
      site = buildSite { name = "워크스페이스" }
      ancestors =
        listOf(
          buildEntity { node = buildFolder { name = "상위" } },
          buildEntity { node = buildFolder { name = "하위" } },
        )
      node = buildDocument { title = "문서" }
    }

    assertEquals(listOf("워크스페이스", "상위", "하위"), entity.breadcrumbNames())
  }

  @Test
  fun `folder breadcrumb names omit blank site names`() {
    val entity = detailsEntity {
      id = "entity-1"
      site = buildSite { name = " " }
      ancestors = listOf(buildEntity { node = buildFolder { name = "상위" } })
      node = buildFolder { name = "폴더" }
    }

    assertEquals(listOf("상위"), entity.breadcrumbNames())
  }

  @Test
  fun `folder item keeps entity id for transfer and folder id for folder mutations`() {
    val entity = rowEntity {
      id = "entity-1"
      depth = 3
      node = buildFolder {
        id = "folder-1"
        name = "프로젝트"
        maxDescendantFoldersDepth = 4
      }
    }

    assertEquals("folder-1", entity.folder?.id)
    assertEquals(
      EntityTransferSource.Folder(
        id = "entity-1",
        title = "프로젝트",
        depth = 3,
        maxDescendantFoldersDepth = 4,
      ),
      entity.toTransferSource(),
    )
  }

  @Test
  fun `entity row scope builds parent meta title and supporting entries in order`() {
    val scope = EntityRowScope()
    val parent =
      searchData {
          hits =
            listOf(
              buildSearchHitDocument {
                document = buildDocument {
                  entity = buildEntity {
                    parent = buildEntity {
                      node = buildFolder {
                        name = "상위 폴더"
                        entity = buildEntity {
                          icon = "folder"
                          iconColor = "blue"
                        }
                      }
                    }
                  }
                }
              }
            )
        }
        .search
        .hits
        .first()
        .searchResultDocument_hit!!
        .document
        .entity
        .entityRowParent_entity
        .parentFolderMeta()

    scope.parentMeta(parent)
    scope.title(title = "제목", subtitle = "부제목", trailingText = "방금 전")
    scope.supporting(text = "요약")

    assertEquals(3, scope.entries.size)
    assertTrue(scope.entries[0] is EntityRowParentMetaEntry)
    assertEquals(
      EntityRowTitleEntry(
        title = EntityRowText.Plain("제목"),
        subtitle = EntityRowText.Plain("부제목"),
        trailingText = "방금 전",
      ),
      scope.entries[1],
    )
    assertEquals(
      EntityRowSupportingEntry(text = EntityRowText.Plain("요약"), maxLines = 1),
      scope.entries[2],
    )
  }

  @Test
  fun `entity row scope keeps annotated supporting text and ignores blank parent meta`() {
    val scope = EntityRowScope()
    val preview = AnnotatedString("강조된 미리보기")
    val expected: List<EntityRowScopeEntry> =
      listOf(EntityRowSupportingEntry(text = EntityRowText.Rich(preview), maxLines = 2))

    scope.parentMeta(null)
    scope.supporting(text = preview, maxLines = 2)

    assertEquals(expected, scope.entries)
  }
}

private fun rowEntity(block: co.typie.graphql.builder.EntityBuilder.() -> Unit): EntityRow_entity =
  SpaceScreen_Query.Data(PlaceholderResolver) {
      site = buildSite { entities = listOf(buildEntity(block)) }
    }
    .site
    .entities
    .first()
    .entityRow_entity

private fun detailsEntity(
  block: co.typie.graphql.builder.EntityBuilder.() -> Unit
): EntityDetails_entity =
  FolderScreen_Query.Data(PlaceholderResolver) { entity = buildEntity(block) }
    .entity
    .entityDetails_entity

private fun searchData(block: co.typie.graphql.builder.SearchResultBuilder.() -> Unit) =
  SearchScreen_Search_Query.Data(PlaceholderResolver) { search = buildSearchResult(block) }
