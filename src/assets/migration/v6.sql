begin;

-- SQLite doesn't support adding columns with foreign keys via ALTER TABLE,
-- so we need to recreate the tables with the new schema
-- Create new Files_Tags table with inheritedfrom column
create table Files_Tags_new (
    fileRecordTagId integer primary key autoincrement,
    fileRecordId integer not null references FileRecords (id) on delete cascade,
    tagId integer not null references Tags (id) on delete cascade,
    inheritedfrom integer references Folders (id) on delete cascade,
    -- Unique constraint on fileRecordId and tagId effectively enforces uniqueness on tag title and file id
    -- since Tags.title is unique and tagId maps 1:1 to title
    unique (fileRecordId, tagId)
);

-- Create new Folders_Tags table with inheritedfrom column
create table Folders_Tags_new (
    foldersTagId integer primary key autoincrement,
    folderId integer not null references Folders (id) on delete cascade,
    tagId integer not null references Tags (id) on delete cascade,
    inheritedfrom integer references Folders (id) on delete cascade,
    -- Unique constraint on folderId and tagId effectively enforces uniqueness on tag title and folder id
    -- since Tags.title is unique and tagId maps 1:1 to title
    unique (folderId, tagId)
);

-- Copy data from old tables to new tables
insert into
    Files_Tags_new(
        fileRecordTagId,
        fileRecordId,
        tagId,
        inheritedfrom
    )
select
    fileRecordTagId,
    fileRecordId,
    tagId,
    null
from
    Files_Tags;

insert into
    Folders_Tags_new(foldersTagId, folderId, tagId, inheritedfrom)
select
    foldersTagId,
    folderId,
    tagId,
    null
from
    Folders_Tags;

-- Drop old tables
drop table Files_Tags;

drop table Folders_Tags;

-- Rename new tables to original names
alter table
    Files_Tags_new rename to Files_Tags;

alter table
    Folders_Tags_new rename to Folders_Tags;

-- Update version
update
    Metadata
set
    value = '6'
where
    name = 'version';

commit;