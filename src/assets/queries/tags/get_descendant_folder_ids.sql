-- Get all descendant folder IDs recursively
with recursive descendants as (
    select id from Folders where parentId = ?1
    union all
    select f.id from Folders f join descendants d on f.parentId = d.id
)
select id from descendants
