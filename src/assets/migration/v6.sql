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


-- inherit all 

commit;