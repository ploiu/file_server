-- Recursively get all descendant folder IDs for a given folder
with recursive descendants(folderId) as (
    -- base case: direct children of the folder
    select id as folderId
    from Folders
    where parentId = ?1
    union all
    -- recursive case: children of children
    select f.id
    from Folders f
    join descendants d on f.parentId = d.folderId
)
select folderId from descendants
