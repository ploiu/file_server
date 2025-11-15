-- create centralized tagged items table to simplify searching and tagging
begin;

create table TaggedItems (
    id integer primary key,
    tagId integer not null references Tags(id) on delete cascade,
    fileId integer references FileRecords(id) on delete cascade,
    folderId integer references Folders(id) on delete cascade,
    -- items can only ever inherit tags from an ancestor folder. When this inherited folder is deleted, this tag should be removed too since it's no longer inherited
    inheritedFromId integer references Folders(id) on delete cascade default null,
    -- make sure that either a file or a folder was tagged
    check ((fileId is not null) != (folderId is not null))
);

-- partial unique to prevent the same tag from being applied to a tagged item
create unique index idx_tagged_items_unique_file on TaggedItems(tagId, fileId)
where
    fileId is not null;

create unique index idx_tagged_items_unique_folder on TaggedItems(tagId, folderId)
where
    folderId is not null;

-- migrate all direct tags for files
insert into
    TaggedItems(tagId, fileId)
select
    tagId,
    fileRecordId
from
    Files_Tags;

-- migrate all direct tags for folders
insert into
    TaggedItems(tagId, folderId)
select
    tagId,
    folderId
from
    Folders_Tags;

/*
 populating inherited tags for folders (needs to be done first so that files work):
 1. recursively get all parent folders along with how far needed to be traveled for that parent folder (depth)
 2. get all tags for all parent folders
 3. for any duplicate tags, take only the ancestor id with the lowest depth (lower depth = higher specificity)
 
 if ai is helpful for anything, it's providing an example that I can adapt while I properly read how recursive sql queries work. 
 Previously, I was flailing around. It helps me if I think of it as a do while loop and temporary named queries / functions 
 */
with recursive 
-- traverse the ancestor tree and track depth
ancestors(folderId, ancestorId, depth) as (
    -- base case: select all folders that have a parent
    select id as folderId, parentId as ancestorId, 1 as depth
    from folders
    where parentId is not null

    union all

    -- iteration: keep retrieving parents from base case until there are no more parents
    select a.folderId, f.parentId as ancestorId, a.depth + 1
    from ancestors a
    join folders f on f.id = a.ancestorId
    where f.parentId is not null
),
-- include all tags with fetched ancestors
ancestorTags as (
    select a.folderId, ft.tagId, a.ancestorId, a.depth
    from ancestors a
    join folders_tags ft on ft.folderId = a.ancestorId
),
-- iterate through all retrieved ancestors. For each entry, find the tag on the ancestor with the lowest depth
nearestTags as (
    select at.folderId, at.tagId, at.ancestorId
    from ancestorTags at
    where at.ancestorId = (
        -- compare on the current row and find the nearest ancestor
        select at2.ancestorId
        from ancestorTags at2
        where at2.folderId = at.folderId 
            and at2.tagId = at.tagId
        order by at2.depth asc
        limit 1
    )
)

-- now that we have our functions, we can invoke nearestTags to get all the inherited tags and insert them
insert into TaggedItems(tagId, folderId, inheritedFromId)
select n.tagId, n.folderId, n.ancestorId
from nearestTags n
-- important to not include tags that are directly on the folder
where not exists (
    select 1 from TaggedItems ti where ti.tagId = n.tagId and ti.folderId = n.folderId
);

-- populate inherited tags for files: for each file, walk its containing folder(s)' ancestor chain
-- and pick the nearest ancestor that provides a tag, then insert an inherited row for the file
with recursive
ancestors(fileId, directFolderId, ancestorId, depth) as (
    -- base: each file's direct containing folder is the first ancestor (so tags on the folder itself are inherited)
    select ff.fileId, ff.folderId, ff.folderId as ancestorId, 1 as depth
    from Folder_Files ff

    union all

    -- climb up the folder parent chain
    select fa.fileId, fa.directFolderId, f.parentId as ancestorId, fa.depth + 1
    from ancestors fa
    join Folders f on f.id = fa.ancestorId
    where f.parentId is not null
),
-- join the discovered ancestors to tags present on those ancestor folders
ancestorTags as (
    select fa.fileId, ft.tagId, fa.ancestorId, fa.depth
    from ancestors fa
    join Folders_Tags ft on ft.folderId = fa.ancestorId
),
-- for each (file,tag) choose the nearest ancestor (smallest depth)
nearestTags as (
    select cft.fileId, cft.tagId, cft.ancestorId
    from ancestorTags cft
    where cft.ancestorId = (
        select cft2.ancestorId
        from ancestorTags cft2
        where cft2.fileId = cft.fileId and cft2.tagId = cft.tagId
        order by cft2.depth asc
        limit 1
    )
)

insert into TaggedItems(tagId, fileId, inheritedFromId)
select n.tagId, n.fileId, n.ancestorId
from nearestTags n
where not exists (
    select 1 from TaggedItems ti where ti.tagId = n.tagId and ti.fileId = n.fileId
);

drop table folders_tags;
drop table files_tags;

update metadata set value = 6 where name = 'version';
commit;