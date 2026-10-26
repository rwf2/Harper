# Unreleased

  * The root content collection is always named `/` in `site.collections`.
    Previously, it was named `""` when `content/` contained an index file.
  * Data directories in `collection.data` are named by their path relative to
    the collection, e.g. `2024/team`. Previously, they were named by their
    file stem, so `v1.2` was `v1`, and same-named directories were ambiguous.
  * `collection.items`, `site.items`, and arrays are sequences in templates.
    They can be sliced (`items[:5]`) and indexed from the end (`items[-1]`),
    and `x in items` tests membership. Previously, they were maps from index
    to value, so slicing failed, `items[-1]` was undefined, and `x in items`
    was always false.

# Version 0.0.3 (Oct 25, 2026)

This is the third testing release of Mockingbird. More details to come.

# Version 0.0.2 (Apr 4, 2024)

This is the second testing release of Mockingbird. More details to come.

# Version 0.0.1 (Mar 3, 2024)

This is the initial testing release of Mockingbird. More details to come.
