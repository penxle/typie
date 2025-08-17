<script lang="ts">
  import { graphql } from '$graphql';

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const query = graphql(`
    query InitialPage_Query {
      me @required {
        id
        preferences

        sites {
          id

          firstEntity(type: POST) {
            id
            slug
          }
        }

        recentlyViewedEntities {
          id
          slug
        }
      }
    }
  `);
</script>
