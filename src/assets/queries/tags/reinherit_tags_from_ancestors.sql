-- Re-inherit tags from ancestor folders after a tag has been removed
-- This is needed when a direct tag is removed from a folder, but the tag should still be inherited
-- from a higher-up folder
-- Parameters: ?1 = folder_id, ?2 = tag_id

-- Re-inherit for the folder itself
insert into Folders_Tags(folderId, tagId, isDirect)
with recursive ancestors(id, level) as (
    select parentId as id, 1 as level
    from Folders
    where id = ?1 and parentId is not null
    union all
    select f.parentId as id, a.level + 1 as level
    from Folders f
    join ancestors a on f.id = a.id
    where f.parentId is not null
)
select ?1, ?2, 0
from ancestors a
join Folders_Tags ft on a.id = ft.folderId
where ft.tagId = ?2 and ft.isDirect = 1
and not exists (
    select 1 from Folders_Tags ft2
    where ft2.folderId = ?1 and ft2.tagId = ?2
)
limit 1;

-- Re-inherit for descendant folders
insert into Folders_Tags(folderId, tagId, isDirect)
with recursive descendants(id) as (
    select id from Folders where parentId = ?1
    union all
    select f.id from Folders f
    join descendants d on f.parentId = d.id
),
ancestors_with_tag as (
    -- Find all ancestor folders that have this tag directly
    with recursive ancestors(id, level) as (
        select parentId as id, 1 as level
        from Folders
        where id = ?1 and parentId is not null
        union all
        select f.parentId as id, a.level + 1 as level
        from Folders f
        join ancestors a on f.id = a.id
        where f.parentId is not null
    )
    select a.id, a.level
    from ancestors a
    join Folders_Tags ft on a.id = ft.folderId
    where ft.tagId = ?2 and ft.isDirect = 1
)
select d.id, ?2, 0
from descendants d
where exists (select 1 from ancestors_with_tag)
and not exists (
    select 1 from Folders_Tags ft
    where ft.folderId = d.id and ft.tagId = ?2
);

-- Re-inherit for files in descendant folders (including the folder itself)
insert into Files_Tags(fileRecordId, tagId, isDirect)
with recursive descendant_folders(id) as (
    -- Start with the folder itself
    select ?1 as id
    union all
    select f.id from Folders f
    join descendant_folders df on f.parentId = df.id
),
ancestors_with_tag as (
    -- Find if any ancestor has this tag directly
    with recursive ancestors(id, level) as (
        select parentId as id, 1 as level
        from Folders
        where id = ?1 and parentId is not null
        union all
        select f.parentId as id, a.level + 1 as level
        from Folders f
        join ancestors a on f.id = a.id
        where f.parentId is not null
    )
    select a.id, a.level
    from ancestors a
    join Folders_Tags ft on a.id = ft.folderId
    where ft.tagId = ?2 and ft.isDirect = 1
)
select ff.fileId, ?2, 0
from Folder_Files ff
join descendant_folders df on ff.folderId = df.id
where exists (select 1 from ancestors_with_tag)
and not exists (
    select 1 from Files_Tags ft
    where ft.fileRecordId = ff.fileId and ft.tagId = ?2
);
