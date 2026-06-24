# Governed artifact reads

The compiled artifacts under the derived directory are read **only** through
`spec-spine` subcommands (`registry`, `index`), never via ad-hoc `jq`/grep over
the JSON. Typed reads make schema drift fail at the deserializer with a clean
error instead of silently encoding stale assumptions.
