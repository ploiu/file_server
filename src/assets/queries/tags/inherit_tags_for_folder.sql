-- Inherit tags from all ancestor folders for a specific folder
-- This finds all tags on ancestor folders and creates inherited tag entries
-- Parameter ?1 is the folder ID
with recursive
-- Get all ancestor folders and track depth
ancestors(folderId, ancestorId, depth) as (
    -- Base case: the folder's direct parent
    select id as folderId, parentId as ancestorId, 1 as depth
    from Folders
    where id = ?1 and parentId is not null

    union all

    -- Recursive: keep climbing up the parent chain
    select a.folderId, f.parentId as ancestorId, a.depth + 1
    from ancestors a
    join Folders f on f.id = a.ancestorId
    where f.parentId is not null
),
-- Join ancestors to tags on those folders (only direct tags, not inherited)
ancestorTags as (
    select a.folderId, ti.tagId, a.ancestorId, a.depth
    from ancestors a
    join TaggedItems ti on ti.folderId = a.ancestorId
    where ti.inheritedFromId is null
),
-- For each (folder, tag) pair, choose the nearest ancestor (smallest depth)
nearestTags as (
    select at.folderId, at.tagId, at.ancestorId
    from ancestorTags at
    where at.ancestorId = (
        select at2.ancestorId
        from ancestorTags at2
        where at2.folderId = at.folderId and at2.tagId = at.tagId
        order by at2.depth asc
        limit 1
    )
)
-- Insert inherited tags, but only if the folder doesn't already have that tag
insert into TaggedItems(tagId, folderId, inheritedFromId)
select n.tagId, n.folderId, n.ancestorId
from nearestTags n
where not exists (
    select 1 from TaggedItems ti 
    where ti.tagId = n.tagId and ti.folderId = n.folderId
);
